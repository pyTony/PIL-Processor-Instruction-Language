use super::*;
use crate::host_io::Output;

#[test]
fn clocked_native_block_budget_stops_at_each_irq_poll_boundary() {
    #[derive(Default)]
    struct Observe(Vec<usize>);
    impl InstructionBackend for Observe {
        fn block(&mut self,_:&Image,_:&mut Cpu,_:&mut Ram,_:&mut crate::address::SegmentState,budget:usize)->Option<(usize,Result<(),crate::ExecError>)>{
            self.0.push(budget);
            None
        }
        fn execute(&mut self,_:&Image,_:&std::rc::Rc<crate::guest::Decoded>,_:&mut Cpu,_:&mut Ram,_:&mut crate::address::SegmentState)->Option<Result<(),crate::ExecError>>{
            None
        }
    }
    // JMP short -2: an infinite loop in the guest, bounded by our step limit.
    let mut p=load(&[0xeb,0xfe],"SPIN.COM").unwrap();
    p.clock_offset=Some(0);
    let mut backend=Observe::default();
    let mut output=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let result=run_with_backend(p,600,&mut output,Some(&mut backend)).unwrap();
    assert_eq!(result.stop,Stop::Budget);
    assert_eq!(backend.0.len(),600);
    assert_eq!(backend.0[0],256);
    assert_eq!(backend.0[255],1);
    assert_eq!(backend.0[256],256);
    assert_eq!(backend.0[511],1);
    assert_eq!(backend.0[512],88);
    assert_eq!(backend.0[599],1);
}


#[test]
fn unsupported_native_sites_skip_hooks_and_preserve_lazy_interpreter_state(){
    struct Unavailable;
    impl InstructionBackend for Unavailable{
        fn may_block(&mut self,_:&Image,_:&Ram,_:u32)->bool{false}
        fn may_execute(&mut self,_:&Image,_:&Ram,_:u32)->bool{false}
        fn block(&mut self,_:&Image,_:&mut Cpu,_:&mut Ram,_:&mut crate::address::SegmentState,_:usize)->Option<(usize,Result<(),crate::ExecError>)>{panic!("unavailable native block must not run")}
        fn execute(&mut self,_:&Image,_:&std::rc::Rc<crate::guest::Decoded>,_:&mut Cpu,_:&mut Ram,_:&mut crate::address::SegmentState)->Option<Result<(),crate::ExecError>>{panic!("unavailable native instruction must not run")}
    }
    let bytes=[0xb9,32,0,0xb8,0,0,0x83,0xc0,1,0xe2,0xfb,0xb8,0,0x4c,0xcd,0x21];
    let mut a=load(&bytes,"LAZY.COM").unwrap();let mut b=a.clone();let mut backend=Unavailable;
    for budget in [1,2,4,17,23].into_iter().cycle().take(40){
        let mut x=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
        let mut y=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
        let left=run_with_backend(a,budget,&mut x,None).unwrap();
        let right=run_with_backend(b,budget,&mut y,Some(&mut backend)).unwrap();
        assert_eq!(left.loaded.cpu,right.loaded.cpu);assert_eq!(left.loaded.image,right.loaded.image);
        assert_eq!(left.loaded.ram.bytes(),right.loaded.ram.bytes());assert_eq!(left.stop,right.stop);
        assert_eq!(left.completed,right.completed);assert_eq!(x.bytes(),y.bytes());
        if matches!(left.stop,Stop::Exit(0)){assert_eq!(left.loaded.cpu.regs[1],0);return;}
        a=left.loaded;b=right.loaded;
    }
    panic!("fixture did not exit")
}

// Independent handwritten Rust workload using Dine's actual VGA controller
// and memory-mapped latch operations. This is a comparison reference, not
// generated guest code; the caption and keyboard path remain in the DOS run.
#[test]
#[ignore = "long-running graphics and interpreter comparison"]
fn rust_mandelbrot_uses_dine_vga_and_matches_the_dos_program() {
    let bytes=include_bytes!("../../programs/dos/MANDVGA.COM");
    let mut p=load(bytes,"MANDVGA.COM").unwrap();
    let start=std::time::Instant::now();
    loop {
        let result=run(p,MAX_STEPS).unwrap();
        match result.stop {Stop::Budget=>p=result.loaded,Stop::WaitingForInput=>{p=result.loaded;break;},s=>panic!("DOS stop: {s:?}")}
    }
    let interpreter=start.elapsed();
    let mut direct=load(bytes,"MANDVGA.COM").unwrap();
    planar::set_mode(&mut direct,0x12,false).unwrap();
    planar::io(&mut direct,0x3ce,false,0x0205,true).unwrap();
    let start=std::time::Instant::now();
    for y in 0..232i32 { for x in 0..320i32 {
        let (cx,cy)=(-9216+42*x,-5040+42*y);
        let (mut zr,mut zi,mut color)=(0i32,0i32,0u8);
        for k in 0..30u8 {
            if zr<=-8192||zr>=8192||zi<=-8192||zi>=8192 {color=1+k%15;break;}
            let (a,b)=((zr*zr)>>12,(zi*zi)>>12);
            if a+b>16384 {color=1+k%15;break;}
            (zr,zi)=(a-b+cx,((zr*zi)>>12)*2+cy);
        }
        let mask=0xc0u16>>((x%4)*2);
        planar::io(&mut direct,0x3ce,false,(mask<<8)|8,true).unwrap();
        let offset=(y as u32)*160+(x as u32)/4;
        for a in [0xa0000+offset,0xa0000+offset+80] {
            direct.ram.load(a,MemWidth::Byte).unwrap();
            direct.ram.store(a,MemWidth::Byte,u32::from(color)).unwrap();
        }
    }}
    let elapsed=start.elapsed();
    for n in 0..4 {
        assert_eq!(&direct.ram.planar().unwrap().plane(n)[..80*464],&p.ram.planar().unwrap().plane(n)[..80*464],"plane {n}");
    }
    println!("DOS interpreter: {interpreter:?}; direct Rust and Dine VGA: {elapsed:?}; 296960 pixels matched");
}

#[derive(Default)]
struct Misses(usize,usize);
impl InstructionBackend for Misses {
    fn execute(&mut self,_:&Image,decoded:&std::rc::Rc<super::super::guest::Decoded>,cpu:&mut Cpu,ram:&mut Ram,machine:&mut super::super::address::SegmentState)->Option<Result<(),super::super::flow::ExecError>> {
        self.0+=1;
        if self.0 % 2 == 0 {
            if let Operation::Pil{program,dst,src,flow,address}=&decoded.operation {
                self.1+=1;
                return Some(crate::execute_machine(program,cpu,*dst,*src,*flow,ram,*address,machine));
            }
        }
        None
    }
}

#[test]
fn backend_misses_preserve_interpreter_results_across_slices() {
    let mut left=load(&hello("com").unwrap(),"HELLO.COM").unwrap();
    let mut right=left.clone();
    let mut a=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let mut b=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    let mut backend=Misses::default();
    for _ in 0..32 {
        let x=run_with_backend(left,1,&mut a,None).unwrap();
        let y=run_with_backend(right,1,&mut b,Some(&mut backend)).unwrap();
        assert_eq!(x.loaded.cpu,y.loaded.cpu);
        assert_eq!(x.loaded.image,y.loaded.image);
        assert_eq!(x.loaded.ram.bytes(),y.loaded.ram.bytes());
        assert_eq!(a.bytes(),b.bytes());
        assert_eq!(x.stop,y.stop);
        assert_eq!(x.completed,y.completed);
        assert_eq!(x.dos_calls,y.dos_calls);
        if matches!(x.stop,Stop::Exit(0)) { assert!(backend.0>backend.1 && backend.1>0); return; }
        assert_eq!(x.stop,Stop::Budget);
        left=x.loaded;right=y.loaded;
    }
    panic!("hello did not exit");
}

#[test]
fn session_retains_backend_across_frontend_slices() {
    use std::{cell::Cell,rc::Rc};
    struct Count(Rc<Cell<usize>>);
    impl InstructionBackend for Count {
        fn execute(&mut self,_:&Image,_:&std::rc::Rc<crate::guest::Decoded>,_:&mut Cpu,_:&mut Ram,_:&mut crate::address::SegmentState)->Option<Result<(),crate::flow::ExecError>> {
            self.0.set(self.0.get()+1);None
        }
    }
    let program=load(&hello("com").unwrap(),"HELLO.COM").unwrap();
    let count=Rc::new(Cell::new(0));
    let mut normal=session::Session::new(program.clone());
    let mut compiled=session::Session::with_backend(program,Box::new(Count(count.clone())));
    for _ in 0..6 {
        assert_eq!(normal.run(1).unwrap(),compiled.run(1).unwrap());
        assert_eq!(normal.loaded().unwrap().cpu,compiled.loaded().unwrap().cpu);
        assert_eq!(normal.screen().unwrap(),compiled.screen().unwrap());
    }
    assert!(count.get()>=3);
}
