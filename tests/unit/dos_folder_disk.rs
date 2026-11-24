use super::*;

fn scratch(tag:&str)->PathBuf{
    let dir=std::env::temp_dir().join(format!("pil-folder-test-{tag}-{}",std::process::id()));
    let _=std::fs::remove_dir_all(&dir);std::fs::create_dir_all(&dir).unwrap();dir
}

/// Put `bytes` at `path` in the volume by rewriting its FAT directory entry
/// and clusters the way a guest would, using only the layout read here.
struct Volume{image:Vec<u8>,base:usize,unit:usize,fat:usize,fat_sectors:usize,root:usize,data:usize}
impl Volume{
    fn new(image:Vec<u8>)->Self{
        let base=u32::from_le_bytes(image[454..458].try_into().unwrap()) as usize*512;
        let b=&image[base..base+512];
        let unit=usize::from(b[13])*512;let fat=base+512*usize::from(u16::from_le_bytes([b[14],b[15]]));
        let fat_sectors=usize::from(u16::from_le_bytes([b[22],b[23]]));
        let root=fat+2*fat_sectors*512;let data=root+usize::from(u16::from_le_bytes([b[17],b[18]]))*32;
        Self{image,base,unit,fat,fat_sectors,root,data}
    }
    fn fat_get(&self,c:usize)->u16{u16::from_le_bytes([self.image[self.fat+2*c],self.image[self.fat+2*c+1]])}
    fn fat_set(&mut self,c:usize,v:u16){for copy in 0..2{let at=self.fat+copy*self.fat_sectors*512+2*c;self.image[at..at+2].copy_from_slice(&v.to_le_bytes());}}
    fn free(&self)->usize{(2..).find(|&c|self.fat_get(c)==0).unwrap()}
    /// Write a new or replaced file of one cluster in the root directory.
    fn put_root(&mut self,name:&[u8;11],bytes:&[u8]){
        assert!(bytes.len()<=self.unit);
        let c=self.free();self.fat_set(c,0xffff);
        let at=self.data+(c-2)*self.unit;self.image[at..at+bytes.len()].copy_from_slice(bytes);
        let mut slot=None;
        for i in 0..512{let e=self.root+i*32;if &self.image[e..e+11]==name||(slot.is_none()&&matches!(self.image[e],0|0xe5)){slot=Some(e);if &self.image[e..e+11]==name{break;}}}
        let e=slot.unwrap();self.image[e..e+11].copy_from_slice(name);self.image[e+11]=0x20;
        self.image[e+26..e+28].copy_from_slice(&(c as u16).to_le_bytes());self.image[e+28..e+32].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
    }
    fn delete_root(&mut self,name:&[u8;11]){
        for i in 0..512{let e=self.root+i*32;if &self.image[e..e+11]==name{self.image[e]=0xe5;}}
    }
}

#[test]
fn folder_becomes_a_partitioned_fat16_disk(){
    let dir=scratch("layout");
    std::fs::write(dir.join("HELLO.TXT"),b"hello\n").unwrap();
    std::fs::write(dir.join("mandel.c"),b"int main(){return 0;}\n").unwrap();
    std::fs::write(dir.join("a long name.text"),vec![7u8;5000]).unwrap();
    std::fs::create_dir(dir.join("src")).unwrap();
    std::fs::write(dir.join("src").join("x.s"),b"ret\n").unwrap();
    std::fs::write(dir.join("empty"),b"").unwrap();
    let built=build(&dir).unwrap();
    let image=&built.image;
    // 65 whole cylinders of 16 heads and 63 sectors.
    assert_eq!(image.len(),65*16*63*512);
    // MBR: one FAT16 partition from LBA 63 (CHS 0/1/1), signature.
    assert_eq!(&image[510..512],&[0x55,0xaa]);
    assert_eq!(&image[446..450],&[0x00,0x01,0x01,0x00]);
    assert_eq!(image[450],0x04);
    assert_eq!(u32::from_le_bytes(image[454..458].try_into().unwrap()),63);
    assert_eq!(u32::from_le_bytes(image[458..462].try_into().unwrap()) as usize,image.len()/512-63);
    // The partition's BPB.
    let b=&image[63*512..64*512];
    assert_eq!(&b[54..62],b"FAT16   ");assert_eq!(u16::from_le_bytes([b[11],b[12]]),512);
    assert_eq!(u32::from_le_bytes(b[28..32].try_into().unwrap()),63);
    // An independent reader finds every file under its 8.3 name.
    let archive=crate::containers::parse(image,false).unwrap();
    let mut names:Vec<(&str,usize)>=archive.files.iter().map(|f|(f.path.as_str(),f.bytes.len())).collect();names.sort();
    assert_eq!(names,vec![("ALONGN~1.TEX",5000),("EMPTY",0),("HELLO.TXT",6),("MANDEL.C",22),("SRC/X.S",4)]);
    assert_eq!(archive.files.iter().find(|f|f.path=="HELLO.TXT").unwrap().bytes,b"hello\n");
    // Nothing to copy back from an untouched disk.
    assert_eq!(write_back(image,&built.snapshot).unwrap(),"0 changed and 0 new files copied back, 0 new directories");
    assert_eq!(std::fs::read(dir.join("HELLO.TXT")).unwrap(),b"hello\n");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn guest_changes_go_back_to_the_folder(){
    let dir=scratch("back");
    std::fs::write(dir.join("keep.txt"),b"same").unwrap();
    std::fs::write(dir.join("edit.txt"),b"old").unwrap();
    std::fs::write(dir.join("race.txt"),b"old").unwrap();
    std::fs::write(dir.join("gone.txt"),b"bye").unwrap();
    let built=build(&dir).unwrap();
    let mut v=Volume::new(built.image);
    assert!(v.base>0);
    v.put_root(b"EDIT    TXT",b"new contents");
    v.put_root(b"RACE    TXT",b"guest");
    v.put_root(b"MANDEL  C  ",b"int main(){}\n");
    v.delete_root(b"GONE    TXT");
    // The host changes race.txt during the run: the guest's copy loses.
    std::fs::write(dir.join("race.txt"),b"host").unwrap();
    let report=write_back(&v.image,&built.snapshot).unwrap();
    assert!(report.starts_with("1 changed and 1 new files copied back, 0 new directories; 1 files deleted on the disk stay in the folder; changed on the host during the run, not overwritten: "),"{report}");
    assert!(report.ends_with("race.txt"),"{report}");
    assert_eq!(std::fs::read(dir.join("edit.txt")).unwrap(),b"new contents");
    assert_eq!(std::fs::read(dir.join("mandel.c")).unwrap(),b"int main(){}\n");
    assert_eq!(std::fs::read(dir.join("race.txt")).unwrap(),b"host");
    assert_eq!(std::fs::read(dir.join("keep.txt")).unwrap(),b"same");
    assert_eq!(std::fs::read(dir.join("gone.txt")).unwrap(),b"bye");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_broken_volume_is_reported_and_the_image_kept(){
    let dir=scratch("broken");
    std::fs::write(dir.join("a.txt"),b"a").unwrap();
    let(disk,_)=FolderDisk::create(&dir).unwrap();
    let mut image=std::fs::read(&disk.image).unwrap();
    image[63*512+510]=0;image[63*512+11]=0; // the guest wrecked the boot sector
    std::fs::write(&disk.image,&image).unwrap();
    let path=disk.image.clone();
    let e=disk.finish().unwrap_err();
    assert!(e.contains("cannot be read back")&&e.contains("stays at"),"{e}");
    assert!(path.exists());std::fs::remove_file(path).unwrap();
    // A good run removes its image.
    let(disk,_)=FolderDisk::create(&dir).unwrap();let path=disk.image.clone();
    disk.finish().unwrap();assert!(!path.exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn not_a_folder_or_too_many_root_entries(){
    let dir=scratch("limits");
    let file=dir.join("f");std::fs::write(&file,b"x").unwrap();
    assert!(build(&file).unwrap_err().contains("not a directory"));
    for i in 0..512{std::fs::write(dir.join(format!("f{i}")),b"").unwrap();}
    assert!(build(&dir).unwrap_err().contains("root directory holds 511"));
    std::fs::remove_dir_all(&dir).unwrap();
}
