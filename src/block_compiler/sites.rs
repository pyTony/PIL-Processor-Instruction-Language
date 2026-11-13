//! Backend-independent instruction sites, effect profiles and block planning.
use crate::{address,Program,View};
use std::collections::{BTreeMap,BTreeSet};
pub(crate) const MAX_BLOCK_INSTRUCTIONS:usize=16;

#[derive(Clone)]
pub(crate) struct Site { pub(crate) signature:Vec<u8>, pub(crate) program:Program, pub(crate) dst:View, pub(crate) src:View, pub(crate) next:u32, pub(crate) flow:crate::FlowContext, pub(crate) address:Option<address::AddressContext>, pub(crate) machine:address::SegmentState, pub(crate) profile:InstructionProfile }
// Conservative capability tags derived from PIL semantics, not just observed
// branch outcomes. They are compile-time choices, not checks in the hot loop.
#[derive(Clone,Copy,Debug)]
pub(crate) struct InstructionProfile {pub(crate) register_only:bool,pub(crate) stores:bool,pub(crate) changes_context:bool,pub(crate) memory:bool}
impl InstructionProfile {
    pub(crate) fn of(p:&Program,address:Option<address::AddressContext>)->Self{
        Self{register_only:address.is_none()&&unpaged_register_only(p),memory:p.uses_memory,
            stores:p.tokens.iter().any(|t|matches!(t,crate::Token::Store(..)|crate::Token::StoreIf(..)|crate::Token::StoreSegment(..))),
            changes_context:p.tokens.iter().any(|t|matches!(t,crate::Token::WriteControl(..)|crate::Token::WriteTable(..)|crate::Token::LoadSystem(..)|crate::Token::FarBranch(..)|crate::Token::FarJump(..)|crate::Token::FarCall(..)|crate::Token::FarReturn(..)|crate::Token::WriteSegment(..)))}
    }
    pub(crate) fn tag(self)->&'static str{if self.changes_context{"machine-context"}else if self.memory||self.stores{"memory"}else if self.register_only{"register-only"}else{"guarded"}}
}
// Primary blocks partition the eligible code. Every interior instruction also
// has a one-instruction alternate entry, rather than a duplicated full suffix.
// This preserves arbitrary entry and small-budget execution in both runtimes.
pub(crate) struct BlockPlan {entries:BTreeMap<u32,Vec<u32>>,primary:usize}
impl BlockPlan {
    pub(crate) fn new(sites:&BTreeMap<u32,Site>,hints:impl IntoIterator<Item=u32>)->Self{
        let eligible:BTreeSet<_>=sites.iter().filter_map(|(&ip,s)|(!s.program.partial).then_some(ip)).collect();
        let mut leaders:BTreeSet<_>=hints.into_iter().filter(|ip|eligible.contains(ip)).collect();
        let mut predecessors:BTreeMap<u32,usize>=BTreeMap::new();
        for &ip in &eligible{
            let site=&sites[&ip];
            let boundary=site.program.uses_flow||site.profile.stores||site.profile.changes_context||site.next<=ip;
            if eligible.contains(&site.next){
                if boundary||site.machine!=sites[&site.next].machine{leaders.insert(site.next);}
                else{*predecessors.entry(site.next).or_default()+=1;}
            }
            if site.program.uses_flow&&site.program.uses_displacement{
                let target=site.next.wrapping_add_signed(site.flow.displacement)&crate::mask(site.program.operand_width)as u32;
                if eligible.contains(&target){leaders.insert(target);}
            }
        }
        for &ip in &eligible{if predecessors.get(&ip)!=Some(&1){leaders.insert(ip);}}
        let mut covered=BTreeSet::new();let mut entries=BTreeMap::new();let mut primary=0;
        for &start in &eligible{
            if covered.contains(&start){continue;}
            primary+=1;leaders.insert(start);
            let mut sequence=Vec::new();let mut ip=start;
            loop{
                let site=&sites[&ip];sequence.push(ip);covered.insert(ip);
                if sequence.len()==MAX_BLOCK_INSTRUCTIONS||site.program.uses_flow||site.profile.stores||
                    site.profile.changes_context||site.next<=ip||!eligible.contains(&site.next)||
                    leaders.contains(&site.next)||covered.contains(&site.next)||site.machine!=sites[&site.next].machine{break;}
                ip=site.next;
            }
            for &ip in sequence.iter().skip(1){entries.insert(ip,vec![ip]);}
            entries.insert(start,sequence);
        }
        Self{entries,primary}
    }
    pub(crate) fn sequence<'a>(&self,sites:&'a BTreeMap<u32,Site>,start:u32)->Vec<(u32,&'a Site)>{
        self.entries.get(&start).into_iter().flatten().map(|&ip|(ip,&sites[&ip])).collect()
    }
    pub(crate) fn contains(&self,ip:u32)->bool{self.entries.contains_key(&ip)}
    pub(crate) fn summary(&self)->String{
        format!("{} primary blocks; {} single-instruction alternate entries; {} emitted instruction bodies",
            self.primary,self.entries.len()-self.primary,self.entries.values().map(Vec::len).sum::<usize>())
    }
}
// These register-only PIL operations cannot touch memory, change the address
// space or produce a partial instruction. In an unpaged block they need no
// per-instruction RAM paging synchronization. Everything else uses the
// ordinary generated wrapper, including instructions that may change mode.
pub(crate) fn unpaged_register_only(p:&Program)->bool{
    !p.uses_memory && !p.partial && !p.checkpoint_flags && p.tokens.iter().all(|t|matches!(t,
        crate::Token::Read(..)|crate::Token::ReadFlag(..)|crate::Token::ReadFlags(..)|
        crate::Token::ReadGpr(..)|crate::Token::WriteGpr(..)|crate::Token::ReadIp(..)|
        crate::Token::ReadDisp(..)|crate::Token::BranchIf(..)|crate::Token::PreserveFlags(..)|
        crate::Token::UndefFlag(..)|crate::Token::Select(..)|crate::Token::WriteIf(..)|
        crate::Token::WriteSource(..)|crate::Token::WriteSourceIf(..)|crate::Token::Const(..)|
        crate::Token::Extend(..)|crate::Token::SignExtend(..)|crate::Token::Truncate(..)|
        crate::Token::Binary(..)|crate::Token::Shift(..)|crate::Token::Less(..)|
        crate::Token::FlagIf(..)|crate::Token::Not(..)|crate::Token::Bit(..)|
        crate::Token::Zero(..)|crate::Token::Parity(..)|crate::Token::Write(..)|
        crate::Token::Flag(..)|crate::Token::Retire))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sites(bytes:&[u8])->BTreeMap<u32,Site>{
        let p=crate::dos::load(bytes,"PLAN.COM").unwrap();let mut sites=BTreeMap::new();let mut ip=0x100;
        while ip<0x100+bytes.len()as u32{
            let decoded=p.image.decode(&p.ram,ip).unwrap();
            if let crate::guest::Operation::Pil{program,dst,src,flow,address}=decoded.operation{
                let profile=InstructionProfile::of(&program,address);
                sites.insert(ip,Site{signature:bytes[(ip-0x100)as usize..(ip-0x100+decoded.length)as usize].to_vec(),program,dst,src,next:flow.next_eip,flow,address,machine:p.image.segment_state(),profile});
            }
            ip+=decoded.length;
        }
        sites
    }
    #[test]
    fn interior_entries_stay_available_without_repeating_full_suffixes(){
        let sites=sites(&[0xb8,1,0,0x40,0x41,0x42,0x43,0xcd,0x20]);
        let plan=BlockPlan::new(&sites,[0x100]);
        assert_eq!(plan.sequence(&sites,0x100).len(),5);
        for ip in 0x103..=0x106{assert_eq!(plan.sequence(&sites,ip).iter().map(|(ip,_)|*ip).collect::<Vec<_>>(),[ip]);}
        assert_eq!(plan.entries.values().map(Vec::len).sum::<usize>(),9); // old suffixes: 15
        let hinted=BlockPlan::new(&sites,[0x100,0x105]);
        assert_eq!(hinted.sequence(&sites,0x100).len(),3);
        assert_eq!(hinted.sequence(&sites,0x105).len(),2);
    }
    #[test]
    fn direct_targets_and_store_boundaries_split_primary_blocks(){
        let sites=sites(&[0xb8,1,0,0x40,0x41,0x75,0xfd,0xc6,6,0,2,1,0x42,0xcd,0x20]);
        let plan=BlockPlan::new(&sites,[0x100]);
        assert_eq!(plan.sequence(&sites,0x100).iter().map(|(ip,_)|*ip).collect::<Vec<_>>(),[0x100,0x103]);
        assert_eq!(plan.sequence(&sites,0x104).iter().map(|(ip,_)|*ip).collect::<Vec<_>>(),[0x104,0x105]);
        assert_eq!(plan.sequence(&sites,0x107).len(),1); // store ends its block
        assert_eq!(plan.sequence(&sites,0x10c).len(),1);
    }
    #[test]
    fn primary_blocks_keep_the_limit_context_and_partial_boundaries(){
        let mut sites=sites(&[vec![0x90;35],vec![0xcd,0x20]].concat());
        let plan=BlockPlan::new(&sites,[0x100]);
        assert_eq!(plan.sequence(&sites,0x100).len(),16);
        assert_eq!(plan.sequence(&sites,0x110).len(),16);
        assert_eq!(plan.sequence(&sites,0x120).len(),3);
        sites.get_mut(&0x102).unwrap().program.partial=true;
        sites.get_mut(&0x105).unwrap().machine.ss_shadow=1;
        let plan=BlockPlan::new(&sites,[0x100]);
        assert_eq!(plan.sequence(&sites,0x100).len(),2);assert!(!plan.contains(0x102));
        assert_eq!(plan.sequence(&sites,0x103).len(),2);
        assert_eq!(plan.sequence(&sites,0x105).len(),1);
    }
}
