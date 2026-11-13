//! Collect the PIL instruction sites a DOS session executes.
//!
//! The recorder is an ordinary `InstructionBackend` that never executes
//! anything: it watches the shared interpreter loop and keeps one `Site` per
//! executed instruction. It does not know which program it is watching. Sites
//! are grouped by the physical base of the code segment they ran in, so a
//! parent and an EXEC child at the same logical offsets stay apart.
use crate::{address,dos,guest,Cpu,ExecError,Ram};
use super::sites::{Site,InstructionProfile};
use std::collections::{BTreeMap,BTreeSet};

/// Sites recorded for one code segment base.
#[derive(Default)]
pub(crate) struct Segment {
    pub(crate) sites:BTreeMap<u32,Site>,
    /// Instructions entered other than by falling through: block leaders.
    pub(crate) entries:BTreeSet<u32>,
    /// Times each instruction was executed.
    pub(crate) visits:BTreeMap<u32,u64>,
}

/// Bounded by `limit` distinct sites in total.
pub(crate) struct Recorder { pub(crate) segments:BTreeMap<u32,Segment>, previous:Option<(u32,u32,bool)>, limit:usize, count:usize }
impl Recorder {
    pub(crate) fn new(limit:usize)->Self{Self{segments:BTreeMap::new(),previous:None,limit,count:0}}
    pub(crate) fn sites(&self)->usize{self.count}
}
impl dos::InstructionBackend for Recorder {
    fn may_block(&mut self,_:&guest::Image,_:&Ram,_:u32)->bool{false}
    fn execute(&mut self,image:&guest::Image,decoded:&std::rc::Rc<guest::Decoded>,cpu:&mut Cpu,ram:&mut Ram,_:&mut address::SegmentState)->Option<Result<(),ExecError>> {
        let base=image.segments[1].base;
        let segment=self.segments.entry(base).or_default();
        *segment.visits.entry(cpu.eip).or_default()+=1;
        if let guest::Operation::Pil{program,dst,src,flow,address}=&decoded.operation {
            if self.previous.is_none_or(|(b,next,transfer)|transfer||b!=base||next!=cpu.eip){segment.entries.insert(cpu.eip);}
            self.previous=Some((base,flow.next_eip,program.uses_flow));
            if self.count>=self.limit||segment.sites.contains_key(&cpu.eip){return None;}
            if let Ok(signature)=image.fetch_signature(ram,cpu.eip,decoded.length){
                segment.sites.insert(cpu.eip,Site{signature:signature[..decoded.length as usize].to_vec(),program:program.clone(),dst:*dst,src:*src,next:flow.next_eip,flow:*flow,address:*address,machine:image.segment_state(),profile:InstructionProfile::of(program,*address)});
                self.count+=1;
            }
        }
        None
    }
}

/// Run `bytes` in an isolated DOS machine (no host drives unless `mount` is
/// given; Z: is an empty memory drive) for at most `steps` steps, replaying
/// `keys` as BIOS keystrokes once startup has had time to settle. Returns the
/// recorder and the stop that ended the run.
pub(crate) fn record(bytes:&[u8],name:&str,keys:&[u8],steps:usize,limit:usize,clock_offset:Option<i64>,mount:Option<&std::path::Path>)->Result<(Recorder,dos::Stop),String>{
    if !(1..=120_000_000).contains(&steps)||!(1..=65_536).contains(&limit){return Err("invalid recording budget or site limit".into());}
    if !crate::containers::executable(name){return Err("expected a DOS .COM or .EXE file".into());}
    if clock_offset.is_some_and(|seconds|seconds.abs()>14*3600){return Err("clock offset must be -50400..50400 seconds".into());}
    let drives=match mount{
        Some(dir)=>vec![dos::Source::Host(dir.to_path_buf()),dos::Source::Memory(dos::MemDir::default())],
        None=>vec![dos::Source::Empty,dos::Source::Memory(dos::MemDir::default())],
    };
    let mut p=dos::load_with_drives(bytes,name,drives,None)?;
    p.clock_offset=clock_offset;
    let mut recorder=Recorder::new(limit);
    let mut output=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
    // One DOS slice is at most MAX_STEPS. Keep the recorder across slices
    // until the program exits, waits for input with no keys left, or the
    // budget ends.
    let mut used=0;let mut key_at=0;
    let stop=loop {
        let ready=keys.is_empty()||used>=steps.min(500_000);
        if ready&&key_at<keys.len(){
            let b=keys[key_at];let scan=if b==13{28}else{0};
            if p.enqueue_key(u16::from(b)|(scan<<8))?{key_at+=1;}
        }
        let quantum=if !ready{50_000}else if key_at<keys.len(){10_000}else{dos::MAX_STEPS};
        let budget=quantum.min(steps-used);
        let result=dos::run_with_backend(p,budget,&mut output,Some(&mut recorder))?;
        used+=budget;
        let waiting=matches!(result.stop,dos::Stop::WaitingForInput);
        if !matches!(result.stop,dos::Stop::Budget|dos::Stop::WaitingForInput)||used==steps||waiting&&key_at==keys.len(){break result.stop;}
        p=result.loaded;
    };
    Ok((recorder,stop))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn records_sites_entries_and_profiles_without_knowing_the_program(){
        // MOV CX,2; INC AX; DEC CX; JZ +5; MOV BX,104h; JMP BX; MOV AH,4C; INT 21
        let bytes=[0xb9,2,0,0x40,0x49,0x74,5,0xbb,4,1,0xff,0xe3,0xb4,0x4c,0xcd,0x21];
        let (r,stop)=record(&bytes,"T.COM",&[],10_000,65_536,None,None).unwrap();
        assert!(matches!(stop,dos::Stop::Exit(_)),"{stop:?}");
        assert_eq!(r.segments.len(),1);
        let s=r.segments.values().next().unwrap();
        // 104h is entered both by falling through and through JMP BX.
        assert!(s.entries.contains(&0x100)&&s.entries.contains(&0x104));
        assert!(!s.entries.contains(&0x103));
        assert_eq!((s.visits[&0x103],s.visits[&0x104]),(1,2));
        assert_eq!(s.sites[&0x100].profile.tag(),"register-only");
        assert_eq!(s.sites[&0x100].signature,[0xb9,2,0]);
    }
    #[test]
    fn site_limit_and_budget_bounds_are_enforced(){
        let bytes=[0x40,0x41,0x42,0xb4,0x4c,0xcd,0x21];
        let (r,_)=record(&bytes,"T.COM",&[],10_000,2,None,None).unwrap();
        assert_eq!(r.sites(),2);
        assert!(record(&bytes,"T.COM",&[],0,2,None,None).is_err());
        assert!(record(&bytes,"README.TXT",&[],10,2,None,None).is_err());
        assert!(record(&bytes,"T.COM",&[],10,2,Some(50_401),None).is_err());
    }
    #[test]
    fn keys_are_replayed_to_a_waiting_program(){
        // MOV AH,0; INT 16h; MOV AH,4C; INT 21h
        let bytes=[0xb4,0,0xcd,0x16,0xb4,0x4c,0xcd,0x21];
        let (_,stop)=record(&bytes,"KEY.COM",b"\r",1_000_000,16,None,None).unwrap();
        assert!(matches!(stop,dos::Stop::Exit(_)),"{stop:?}");
        let (_,stop)=record(&bytes,"KEY.COM",&[],1_000_000,16,None,None).unwrap();
        assert!(matches!(stop,dos::Stop::WaitingForInput),"{stop:?}");
    }
}
