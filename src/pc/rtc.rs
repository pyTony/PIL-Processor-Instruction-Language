//! Session-local CMOS clock. Calendar-year writes never change the host clock.
//! INT 1Ah AH=03h/05h set its time and date the same way. Other RTC write
//! The SET bit of status register B freezes the clock registers: writes
//! while it is set are staged and take effect together when it is cleared,
//! as an OS's clock program sets the time. Other status B modes (binary,
//! 12-hour) and RTC interrupts remain unsupported.
use std::time::{SystemTime,UNIX_EPOCH};
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub(super) struct Rtc{adjust_seconds:i64,weekday_adjust:i64,
    /// While status B's SET bit is on: the clock registers as frozen and
    /// written (second, minute, hour, weekday, day, month, year, century).
    frozen:Option<[u8;8]>,
    /// Status A's divider-chain reset (DV 11xb): Linux holds it around a
    /// clock write. The clock is not held for those few instructions.
    divider_reset:bool}
/// The position of a clock register in a frozen set.
fn slot(index:u8)->Option<usize>{Some(match index{0=>0,2=>1,4=>2,6=>3,7=>4,8=>5,9=>6,0x32=>7,_=>return None})}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
struct Date{year:i64,month:i64,day:i64,weekday:i64,seconds:i64}
// Gregorian civil-date conversion, with signed days relative to 1970-01-01.
fn civil(days:i64)->(i64,i64,i64){
    let z=days+719_468;let era=z.div_euclid(146_097);let doe=z-era*146_097;
    let yoe=(doe-doe/1460+doe/36_524-doe/146_096)/365;let doy=doe-(365*yoe+yoe/4-yoe/100);
    let mp=(5*doy+2)/153;let day=doy-(153*mp+2)/5+1;let month=if mp<10{mp+3}else{mp-9};
    (yoe+era*400+i64::from(month<=2),month,day)
}
fn days(year:i64,month:i64,day:i64)->i64{
    let y=year-i64::from(month<=2);let era=y.div_euclid(400);let yoe=y-era*400;
    let doy=(153*(if month>2{month-3}else{month+9})+2)/5+day-1;
    era*146_097+yoe*365+yoe/4-yoe/100+doy-719_468
}
fn valid(year:i64,month:i64,day:i64)->bool{
    let leap=year%4==0&&(year%100!=0||year%400==0);
    let limit=match month{1|3|5|7|8|10|12=>31,4|6|9|11=>30,2 if leap=>29,2=>28,_=>0};
    (1..=9999).contains(&year)&&day>=1&&day<=limit
}
fn bcd(value:u8)->Option<i64>{(value&15<10&&value>>4<10).then(||i64::from(value>>4)*10+i64::from(value&15))}
fn base(offset:Option<i64>)->i64{
    match offset{Some(offset)=>SystemTime::now().duration_since(UNIX_EPOCH).map_or(0,|t|t.as_secs()as i64)+offset,None=>315_532_800}
}
/// DOS keeps its own date, advanced against the session's day counter.
pub(super) fn dos_date(offset:Option<i64>,adjust_seconds:i64)->(u32,u32,u32,u32){
    dos_date_at(base(offset),adjust_seconds)
}
fn dos_date_at(now:i64,adjust_seconds:i64)->(u32,u32,u32,u32){
    let n=(now+adjust_seconds).div_euclid(86400);let(y,m,d)=civil(n);
    (y as u32,m as u32,d as u32,(n+4).rem_euclid(7)as u32)
}
impl Rtc{
    fn date_at(&self,base:i64)->Date{
        let seconds=base+self.adjust_seconds;let day_count=seconds.div_euclid(86400);
        let(year,month,day)=civil(day_count);
        Date{year,month,day,weekday:(day_count+4+self.weekday_adjust).rem_euclid(7)+1,seconds:seconds.rem_euclid(86400)}
    }
    /// BCD, 24-hour registers, with Sunday=1. Caller handles non-clock CMOS.
    pub fn read(&self,offset:Option<i64>,index:u8)->Option<u8>{
        if let (Some(regs),Some(at))=(self.frozen,slot(index)){return Some(regs[at]);}
        let d=self.date_at(base(offset));let value=match index{0=>d.seconds%60,2=>d.seconds/60%60,4=>d.seconds/3600,6=>d.weekday,7=>d.day,8=>d.month,9=>d.year%100,0x32=>d.year/100,_=>return None};
        Some(((value/10)<<4|value%10)as u8)
    }
    /// One coherent RTC sample, in BCD: century, year, month, day, hour, minute, second.
    pub fn bios(&self,offset:Option<i64>)->[u8;7]{self.bios_at(base(offset))}
    fn bios_at(&self,now:i64)->[u8;7]{
        let d=self.date_at(now);
        [d.year/100,d.year%100,d.month,d.day,d.seconds/3600,d.seconds/60%60,d.seconds%60]
            .map(|v|((v/10)<<4|v%10)as u8)
    }
    fn write_year_at(&mut self,now:i64,index:u8,value:u8)->Result<(),String>{
        if value&15>9||value>>4>9{return Err(format!("CMOS year value {value:02X}h is not valid BCD"));}
        let d=self.date_at(now);let number=i64::from(value>>4)*10+i64::from(value&15);
        let year=match index{9=>d.year/100*100+number,0x32=>number*100+d.year%100,_=>return Err("not a CMOS year register".into())};
        if !valid(year,d.month,d.day){return Err(format!("CMOS date {year:04}-{:02}-{:02} is not valid",d.month,d.day));}
        let before=days(d.year,d.month,d.day);let after=days(year,d.month,d.day);
        self.adjust_seconds+=(after-before)*86400;self.weekday_adjust=(self.weekday_adjust+before-after).rem_euclid(7);Ok(())
    }
    /// Status register B: BCD, 24-hour, no interrupts; bit 7 is SET.
    /// Status A: 32.768 kHz time base (DV 010b, or the reset written),
    /// rate 6 (1024 Hz); update in progress (bit 7) never shows.
    pub fn status_a(&self)->u8{if self.divider_reset{0x66}else{0x26}}
    pub fn status_b(&self)->u8{0x02|if self.frozen.is_some(){0x80}else{0}}
    /// A CPU write to a clock register or status B (other CMOS registers
    /// are the caller's). Nothing changes when the write is refused.
    pub fn write(&mut self,offset:Option<i64>,index:u8,value:u8)->Result<(),String>{self.write_at(base(offset),index,value)}
    fn write_at(&mut self,now:i64,index:u8,value:u8)->Result<(),String>{
        if index==0x0a{
            // Rate 6, divider normal (010b) or held in reset (11xb).
            if value&0x0f!=6||!matches!(value>>4&7,2|6|7){return Err(format!("CMOS status A value {value:02X}h is not modelled (32.768 kHz base, rate 6 only)"));}
            self.divider_reset=value>>4&7!=2;return Ok(());
        }
        if index==0x0b{
            if value&0x7f!=0x02{return Err(format!("CMOS status B value {value:02X}h is not modelled (BCD, 24-hour, no RTC interrupts only)"));}
            match(value&0x80!=0,self.frozen){
                (true,None)=>{let d=self.date_at(now);let b=self.bios_at(now);self.frozen=Some([b[6],b[5],b[4],d.weekday as u8,b[3],b[2],b[1],b[0]]);},
                (false,Some(regs))=>{let mut next=self.clone();next.frozen=None;next.commit(now,regs)?;*self=next;},
                _=>{},
            }
            return Ok(());
        }
        let at=slot(index).ok_or_else(||format!("CMOS write to register {index:02x}h is not modelled"))?;
        if let Some(regs)=self.frozen.as_mut(){regs[at]=value;return Ok(());}
        if matches!(index,9|0x32){return self.write_year_at(now,index,value);}
        // Outside SET a write takes effect at once, the other fields kept.
        let d=self.date_at(now);let b=self.bios_at(now);
        let mut regs=[b[6],b[5],b[4],d.weekday as u8,b[3],b[2],b[1],b[0]];regs[at]=value;
        let mut next=self.clone();next.commit(now,regs)?;*self=next;Ok(())
    }
    fn commit(&mut self,now:i64,r:[u8;8])->Result<(),String>{
        let weekday=bcd(r[3]).filter(|w|(1..=7).contains(w)).ok_or_else(||format!("RTC weekday {:02X}h is not 1-7",r[3]))?;
        self.set_bios_date_at(now,r[7],r[6],r[5],r[4])?;
        self.set_bios_time_at(now,r[2],r[1],r[0])?;
        let day_count=(now+self.adjust_seconds).div_euclid(86400);
        self.weekday_adjust=(weekday-1-(day_count+4)).rem_euclid(7);Ok(())
    }
    /// INT 1Ah AH=03h: set the RTC's time of day (BCD hour, minute, second)
    /// and keep its date; the session-local clock then runs on from there.
    /// DOS's own clock and the BIOS tick count are separate and unchanged.
    pub fn set_bios_time(&mut self,offset:Option<i64>,hour:u8,minute:u8,second:u8)->Result<(),String>{self.set_bios_time_at(base(offset),hour,minute,second)}
    fn set_bios_time_at(&mut self,now:i64,hour:u8,minute:u8,second:u8)->Result<(),String>{
        let [h,m,s]=[hour,minute,second].map(bcd);
        match (h,m,s){(Some(h),Some(m),Some(s)) if h<24&&m<60&&s<60=>{
            let d=self.date_at(now);self.adjust_seconds+=(h*60+m)*60+s-d.seconds;Ok(())
        },_=>Err(format!("RTC time {hour:02X}:{minute:02X}:{second:02X} is not a valid BCD time"))}
    }
    /// INT 1Ah AH=05h: set the RTC's date (BCD century, year, month, day)
    /// and keep its time of day; the day of the week follows the new date.
    pub fn set_bios_date(&mut self,offset:Option<i64>,century:u8,year:u8,month:u8,day:u8)->Result<(),String>{self.set_bios_date_at(base(offset),century,year,month,day)}
    fn set_bios_date_at(&mut self,now:i64,century:u8,year:u8,month:u8,day:u8)->Result<(),String>{
        let [c,y,m,dd]=[century,year,month,day].map(bcd);
        let (Some(c),Some(y),Some(m),Some(dd))=(c,y,m,dd) else{return Err(format!("RTC date {century:02X}{year:02X}-{month:02X}-{day:02X} is not valid BCD"));};
        if !valid(c*100+y,m,dd){return Err(format!("RTC date {century:02X}{year:02X}-{month:02X}-{day:02X} is not a valid date"));}
        let d=self.date_at(now);
        self.adjust_seconds+=(days(c*100+y,m,dd)-days(d.year,d.month,d.day))*86400;self.weekday_adjust=0;Ok(())
    }
    /// Today's month and day in `year` (28 February for 29 February in a
    /// common year), for a clock that runs in another year.
    pub fn date_in_year(&self,offset:Option<i64>,year:u32)->(u32,u32,u32){self.date_in_year_at(base(offset),year)}
    fn date_in_year_at(&self,now:i64,year:u32)->(u32,u32,u32){
        let d=self.date_at(now);let year=i64::from(year);
        let day=if valid(year,d.month,d.day){d.day}else{d.day-1};
        (year as u32,d.month as u32,day as u32)
    }
    /// Set the RTC and DOS calendar date, preserving their time of day.
    pub fn set_dos_date(&mut self,offset:Option<i64>,adjust:i64,year:u32,month:u32,day:u32)->i64{
        let now=base(offset);let d=self.date_at(now);let before=days(d.year,d.month,d.day);let after=days(i64::from(year),i64::from(month),i64::from(day));
        self.adjust_seconds+=(after-before)*86400;self.weekday_adjust=0;
        adjust+(after-(now+adjust).div_euclid(86400))*86400
    }
    pub fn set_dos_time(&mut self,offset:Option<i64>,adjust:i64,seconds:u32)->i64{
        let now=base(offset);let d=self.date_at(now);self.adjust_seconds+=i64::from(seconds)-d.seconds;
        adjust+i64::from(seconds)-(now+adjust).rem_euclid(86400)
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn a_clock_in_another_year_keeps_the_month_day_and_time(){
        let mut r=Rtc::default();
        let now=days(2026,11,23)*86400+3600;
        let (y,m,d)=r.date_in_year_at(now,1993);assert_eq!((y,m,d),(1993,11,23));
        let before=r.date_at(now);
        r.adjust_seconds+=(days(1993,11,23)-days(2026,11,23))*86400;
        assert_eq!(r.bios_at(now),[0x19,0x93,0x11,0x23,0x01,0,0]);
        assert_eq!(r.date_at(now).seconds,before.seconds);
        let leap=days(2024,2,29)*86400;assert_eq!(Rtc::default().date_in_year_at(leap,1993),(1993,2,28));
    }
    #[test]fn bios_snapshot_rolls_over_the_whole_date_at_midnight(){
        let r=Rtc::default();let last=days(2024,2,29)*86400+86399;
        assert_eq!(r.bios_at(last),[0x20,0x24,0x02,0x29,0x23,0x59,0x59]);
        assert_eq!(r.bios_at(last+1),[0x20,0x24,0x03,0x01,0,0,0]);
        assert_eq!(r.bios_at(days(1999,12,31)*86400+86400),[0x20,0,1,1,0,0,0]);
    }
    #[test]fn bios_time_and_date_sets_move_only_the_rtc_fields_they_name(){
        let now=days(2026,11,16)*86400+12345;let mut r=Rtc::default();
        r.set_bios_time_at(now,0x23,0x59,0x59).unwrap();
        assert_eq!(r.bios_at(now),[0x20,0x26,0x11,0x16,0x23,0x59,0x59]);
        assert_eq!(r.bios_at(now+1),[0x20,0x26,0x11,0x17,0,0,0],"the date rolls over a second later");
        r.set_bios_date_at(now,0x19,0x99,0x12,0x31).unwrap();
        assert_eq!(r.bios_at(now),[0x19,0x99,0x12,0x31,0x23,0x59,0x59]);
        assert_eq!(r.bios_at(now+1),[0x20,0,1,1,0,0,0]);
        assert_eq!(r.date_at(now).weekday,6,"31 December 1999 was a Friday (Sunday=1)");
        let old=r.clone();
        assert!(r.set_bios_time_at(now,0x24,0,0).is_err());assert!(r.set_bios_time_at(now,0x1a,0,0).is_err());
        assert!(r.set_bios_date_at(now,0x20,0x25,0x02,0x29).is_err());assert!(r.set_bios_date_at(now,0x20,0x26,0x0a,1).is_err());
        assert_eq!(r,old);
    }
    #[test]fn the_set_bit_stages_clock_writes_until_it_is_cleared(){
        let now=days(2026,11,23)*86400+3600;let mut r=Rtc::default();
        assert_eq!(r.status_b(),0x02);
        r.write_at(now,0x0b,0x82).unwrap();assert_eq!(r.status_b(),0x82);
        // A clock program writes field by field; 31 January passes through
        // 31 November without complaint while frozen.
        for (index,value) in [(0,0x30),(2,0x15),(4,0x10),(6,0x02),(7,0x31),(8,0x01),(9,0x94),(0x32,0x19)]{r.write_at(now,index,value).unwrap();}
        assert_eq!(r.read(None,7),Some(0x31));
        assert_eq!(r.bios_at(now+5),[0x20,0x26,0x11,0x23,0x01,0,5],"the clock keeps its time until SET clears");
        r.write_at(now,0x0b,0x02).unwrap();
        assert_eq!(r.bios_at(now),[0x19,0x94,0x01,0x31,0x10,0x15,0x30]);
        assert_eq!(r.bios_at(now+30),[0x19,0x94,0x01,0x31,0x10,0x16,0]);
        assert_eq!(r.date_at(now).weekday,2,"the weekday written (31 January 1994 was a Monday)");
        // An invalid date written under SET is refused when SET clears, and
        // the clock stays frozen as it was.
        r.write_at(now,0x0b,0x82).unwrap();r.write_at(now,7,0x30).unwrap();r.write_at(now,8,0x02).unwrap();
        let old=r.clone();assert!(r.write_at(now,0x0b,0x02).is_err());assert_eq!(r,old);
        // Binary or 12-hour mode and RTC interrupts are not modelled.
        // Status A: the divider reset Linux writes around its update.
        r.write_at(now,0x0a,0x76).unwrap();assert_eq!(r.status_a(),0x66);
        r.write_at(now,0x0a,0x26).unwrap();assert_eq!(r.status_a(),0x26);
        for v in [0x27,0x06,0x46]{assert!(r.write_at(now,0x0a,v).is_err());}
        let mut q=Rtc::default();for v in [0x06,0x00,0x12,0x42]{assert!(q.write_at(now,0x0b,v).is_err());}
        // Outside SET a single field takes effect at once.
        q.write_at(now,2,0x45).unwrap();assert_eq!(q.bios_at(now)[5],0x45);
        assert!(q.write_at(now,0x0c,0).is_err());
    }
    #[test]fn calendar_year_writes_preserve_other_fields_and_advance_at_the_new_rate(){
        let now=days(2026,11,7)*86400+12345;let mut r=Rtc::default();let old=r.date_at(now);
        r.write_year_at(now,9,0x27).unwrap();let next=r.date_at(now);assert_eq!((next.year,next.month,next.day,next.weekday,next.seconds),(2027,old.month,old.day,old.weekday,old.seconds));
        assert_eq!(r.date_at(now+86400).day,8);r.write_year_at(now,9,0x26).unwrap();assert_eq!(r.date_at(now),old);
        r.write_year_at(now,0x32,0x19).unwrap();assert_eq!(r.date_at(now).year,1926);r.write_year_at(now,0x32,0x20).unwrap();assert_eq!(r.date_at(now),old);
        let old=r.clone();assert!(r.write_year_at(now,9,0xfa).is_err());assert_eq!(r,old);
        let leap=days(2024,2,29)*86400;let mut r=Rtc::default();assert!(r.write_year_at(leap,9,0x25).is_err());assert_eq!(r,Rtc::default());
    }
    #[test]fn dos_date_tracks_guest_midnight_and_setting_date_updates_weekday(){
        let now=days(2026,11,7)*86400+12*3600;
        let adjust=23*3600+59*60+59-12*3600;
        assert_eq!(dos_date_at(now,adjust),(2026,11,7,6));
        assert_eq!(dos_date_at(now+1,adjust),(2026,11,8,0));
        let mut r=Rtc::default();let adjust=r.set_dos_date(None,0,2026,11,8);
        assert_eq!(dos_date(None,adjust),(2026,11,8,0));assert_eq!(r.read(None,6),Some(1));
    }
}
