use super::*;
#[test]
fn invalid_parameters_are_rejected_before_artifacts(){
    for(size,iterations)in [(0,1),(1,1),(3,16),(512,16),(8,0),(8,65)]{assert!(guest_bytes(size,iterations).is_err());assert!(reference(size,iterations).is_err());assert!(render_files(size,iterations).is_err());}
}
#[test]
fn reference_contains_known_boundary_and_escape_pixels(){
    assert_eq!(reference(2,4).unwrap(),[4,8,0,0]);
    let p=reference(8,16).unwrap();assert_eq!(p.len(),64);assert!(p.contains(&0));assert!(p.iter().any(|&x|x>0));
    assert!(reference(256,32).unwrap().iter().all(|&x|x%4==0));
}
#[test]
fn actual_guest_pixels_match_reference_and_tx_backpressure_recovers(){
    for(size,iterations)in [(2,1),(2,4),(4,8),(8,16),(32,4)]{
        let result=super::super::debugger::render_mandelbrot(size,iterations).unwrap();assert_eq!(result.pixels,reference(size,iterations).unwrap());assert!(result.steps>u64::from(size*size));
        assert_eq!(result.blocked,(size*size-1)/256);
    }
}
#[test]
fn bitmap_has_valid_headers_bottom_up_rows_and_padding(){
    let bmp=bitmap(2,&[1,2,3,4]).unwrap();assert_eq!(&bmp[..2],b"BM");assert_eq!(bmp.len(),70);assert_eq!(&bmp[26..30],&[1,0,24,0]);
    assert_eq!(&bmp[54..62],&[3,3,3,4,4,4,0,0]);assert_eq!(&bmp[62..70],&[1,1,1,2,2,2,0,0]);assert!(bitmap(2,&[1]).is_err());
}
