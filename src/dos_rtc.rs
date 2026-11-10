//! Session-local CMOS clock. Calendar-year writes never change the host clock.
//! Other RTC write modes, interrupt generation and SET/freeze remain unsupported.
use std::time::{SystemTime,UNIX_EPOCH};
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub(super) struct Rtc{adjust_seconds:i64,weekday_adjust:i64}
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
    pub fn write_year(&mut self,offset:Option<i64>,index:u8,value:u8)->Result<(),String>{self.write_year_at(base(offset),index,value)}
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
    #[test]fn bios_snapshot_rolls_over_the_whole_date_at_midnight(){
        let r=Rtc::default();let last=days(2024,2,29)*86400+86399;
        assert_eq!(r.bios_at(last),[0x20,0x24,0x02,0x29,0x23,0x59,0x59]);
        assert_eq!(r.bios_at(last+1),[0x20,0x24,0x03,0x01,0,0,0]);
        assert_eq!(r.bios_at(days(1999,12,31)*86400+86400),[0x20,0,1,1,0,0,0]);
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
