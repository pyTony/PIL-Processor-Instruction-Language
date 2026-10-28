use super::*;
#[test]
fn service_keeps_guest_bytes_separate_and_preserves_cli_results(){
 let commands=b"demo hello\nrun256\nrun 256\nregs\nquit\n";let mut out=vec![];serve(&commands[..],&mut out,None).unwrap();let text=String::from_utf8(out).unwrap();let lines:Vec<_>=text.lines().collect();assert_eq!(lines.len(),6);assert!(lines[2].contains("\"ok\":false"));assert!(lines[3].contains("\"status\":\"halt\""));assert!(lines[3].contains("\"guest_output\":[72,101,108,108,111,44,32,119,111,114,108,100,33,10]"));assert!(!lines[3].contains("serial output:"));assert!(lines[4].contains("\"guest_output\":[]"));assert!(lines[5].contains("\"quit\":true"));
 let mut d=Debugger::new("hello").unwrap();d.run(256).unwrap();assert!(lines[4].contains(&quote(&d.command("regs").unwrap().0)));
}
#[test]
fn service_bounds_commands_and_does_not_mutate_on_rejection(){
 let mut d=Debugger::new("counter").unwrap();let old=d.cpu.clone();let mut view=0x100;
 for input in ["run 257","run 0","bench 10000","time 1000000","view 999999","dosprobe a.com","load a.bin"]{assert!(command(&mut d,input,&mut view).is_err());assert_eq!(d.cpu,old);}
 let mut out=vec![];serve(&vec![b'x';4097][..],&mut out,None).unwrap();let text=String::from_utf8(out).unwrap();assert_eq!(text.lines().count(),2);assert!(text.contains("\"quit\":true"));
}
#[test]
fn service_reports_input_wait_binary_zero_and_precise_stop(){
 let mut d=Debugger::new("echo").unwrap();let mut view=0x100;assert_eq!(command(&mut d,"run",&mut view).unwrap().2,"waiting_rx");assert_eq!(d.cpu.eip,0x200);command(&mut d,"serial rx 00",&mut view).unwrap();assert_eq!(command(&mut d,"run",&mut view).unwrap().2,"halt");assert_eq!(d.tx_events,[0]);assert_eq!(d.cpu.eip,0x202);
}
#[test]
fn json_text_is_lossless_for_control_characters_and_quoting(){
 assert_eq!(quote("a\"\\\n\r\t\0\u{1f}ä"),"\"a\\\"\\\\\\n\\r\\t\\u0000\\u001fä\"");
}
