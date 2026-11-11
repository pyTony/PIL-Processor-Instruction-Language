//! Backend-neutral recognition of arithmetic idioms in validated PIL dataflow.
use crate::{Binary,Program,Token};
#[derive(Clone,Copy)]
pub(crate) enum CarryOp { Add,Sub }
pub(crate) struct CarryWindow {
    pub(crate) partial:usize,pub(crate) wide:usize,pub(crate) result:usize,pub(crate) cf:usize,
    pub(crate) a:usize,pub(crate) b:usize,pub(crate) carry:usize,pub(crate) width:u8,pub(crate) op:CarryOp,
}

// A zero extension does not give its input additional significant bits.
fn fits(p:&Program,id:usize,bits:u8)->bool {
    p.widths[id]<=bits||p.tokens.iter().any(|t|matches!(*t,
        Token::Extend(out,input,_) if out==id&&p.widths[input]<=bits))
}

/// Recognize widened (a +/- b) +/- carry followed by low-result/carry
/// extraction. Describe the result/carry outputs independently for a backend
/// to lower or leave unchanged. No source text, opcode names or host instructions
/// are involved. The renderer must retain original SSA values for other users.
pub(crate) fn carry_window(p:&Program,at:usize)->Option<CarryWindow>{
    let [Token::Binary(partial,op,a,b),Token::Binary(wide,op2,input,carry),
         Token::Truncate(result,low,w),Token::Bit(cf,high,bit)] = p.tokens.get(at..at+4)? else{return None;};
    let op=match (op,op2){(Binary::Add,Binary::Add)=>CarryOp::Add,(Binary::Sub,Binary::Sub)=>CarryOp::Sub,_=>return None};
    if input!=partial||low!=wide||high!=wide||bit!=w||!matches!(w,8|16|32)||
        p.widths[*partial]!=w+1||p.widths[*wide]!=w+1||
        !fits(p,*a,*w)||!fits(p,*b,*w)||!fits(p,*carry,1){return None;}
    Some(CarryWindow{partial:*partial,wide:*wide,result:*result,cf:*cf,a:*a,b:*b,carry:*carry,width:*w,op})
}
