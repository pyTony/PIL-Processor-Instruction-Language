//! Text boundary: host Unicode <-> the runtime's fixed DOS code page 437.
//! ANSI controls remain ASCII; instruction semantics and guest RAM stay bytes.
const HIGH:[char;128]=[
    '\u{c7}','\u{fc}','\u{e9}','\u{e2}','\u{e4}','\u{e0}','\u{e5}','\u{e7}',
    '\u{ea}','\u{eb}','\u{e8}','\u{ef}','\u{ee}','\u{ec}','\u{c4}','\u{c5}',
    '\u{c9}','\u{e6}','\u{c6}','\u{f4}','\u{f6}','\u{f2}','\u{fb}','\u{f9}',
    '\u{ff}','\u{d6}','\u{dc}','\u{a2}','\u{a3}','\u{a5}','\u{20a7}','\u{192}',
    '\u{e1}','\u{ed}','\u{f3}','\u{fa}','\u{f1}','\u{d1}','\u{aa}','\u{ba}',
    '\u{bf}','\u{2310}','\u{ac}','\u{bd}','\u{bc}','\u{a1}','\u{ab}','\u{bb}',
    '\u{2591}','\u{2592}','\u{2593}','\u{2502}','\u{2524}','\u{2561}','\u{2562}','\u{2556}',
    '\u{2555}','\u{2563}','\u{2551}','\u{2557}','\u{255d}','\u{255c}','\u{255b}','\u{2510}',
    '\u{2514}','\u{2534}','\u{252c}','\u{251c}','\u{2500}','\u{253c}','\u{255e}','\u{255f}',
    '\u{255a}','\u{2554}','\u{2569}','\u{2566}','\u{2560}','\u{2550}','\u{256c}','\u{2567}',
    '\u{2568}','\u{2564}','\u{2565}','\u{2559}','\u{2558}','\u{2552}','\u{2553}','\u{256b}',
    '\u{256a}','\u{2518}','\u{250c}','\u{2588}','\u{2584}','\u{258c}','\u{2590}','\u{2580}',
    '\u{3b1}','\u{df}','\u{393}','\u{3c0}','\u{3a3}','\u{3c3}','\u{b5}','\u{3c4}',
    '\u{3a6}','\u{398}','\u{3a9}','\u{3b4}','\u{221e}','\u{3c6}','\u{3b5}','\u{2229}',
    '\u{2261}','\u{b1}','\u{2265}','\u{2264}','\u{2320}','\u{2321}','\u{f7}','\u{2248}',
    '\u{b0}','\u{2219}','\u{b7}','\u{221a}','\u{207f}','\u{b2}','\u{25a0}','\u{a0}'
];

pub(crate) fn decode(bytes:&[u8])->String{
    bytes.iter().map(|&b|if b<128{char::from(b)}else{HIGH[usize::from(b-128)]}).collect()
}
pub(crate) fn command_line(text:&str)->Result<Vec<u8>,String>{
    let mut bytes=Vec::new();
    for c in text.chars(){
        let b=if (' '..='~').contains(&c){c as u8}
            else{HIGH.iter().position(|&x|x==c).map(|i|i as u8+128)
                .ok_or_else(||format!("command line character U+{:04X} cannot be represented in DOS CP437",c as u32))?};
        bytes.push(b);
        if bytes.len()>125{return Err("command line is longer than 125 DOS bytes".into());}
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn extended_bytes_round_trip_and_ascii_controls_are_preserved(){
        for b in 128..=255u8{let text=decode(&[b]);assert_eq!(command_line(&text).unwrap(),[b]);}
        assert_eq!(decode(b"\x1b[31m\r\n"),"\x1b[31m\r\n");
        assert_eq!(command_line("tämä on demo").unwrap(),b"t\x84m\x84 on demo");
        assert_eq!(decode(b"t\x84m\x84"),"tämä");
    }
    #[test] fn bounds_count_dos_bytes_and_unmappable_characters_fail(){
        assert_eq!(command_line(&"ä".repeat(125)).unwrap(),vec![0x84;125]);
        for text in [&"ä".repeat(126),"bad\nline","🙂","€"]{assert!(command_line(text).is_err());}
    }
}
