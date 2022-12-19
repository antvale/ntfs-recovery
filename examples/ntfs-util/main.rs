
mod sector_reader;
use std::collections::{HashMap, BTreeMap};
use std::fmt::format;
use std::fs::{File, OpenOptions, create_dir, create_dir_all};
use std::io::{self, prelude::*, BufReader, Read, Seek, Write, Lines};
use std::num::NonZeroU64;

//use anyhow::{anyhow, bail, Context, Result};

use ntfs::{NtfsError, Result};
use ntfs::indexes::NtfsFileNameIndex;
use ntfs::structured_values::{
    NtfsAttributeList, NtfsFileName, NtfsFileNamespace, NtfsStandardInformation,
};
use ntfs::{Ntfs, NtfsAttribute, NtfsAttributeType, NtfsFile, NtfsReadSeek, NtfsTime};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use std::io::prelude::*;

use sector_reader::SectorReader;

use serde::{Serialize, Deserialize};

use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
struct FileInfo {
    record_number: u64,
    record_parent_number: u64,
    file_name: String,
    length: u64,
    path: String,
    access_time:String,
    creation_time:String,
    modification_time:String
}

struct NtfsFolderName<'n> {
    ntfs_file: NtfsFile<'n>,
    ntfs_file_name: String
}

struct NtfsInfo<'n, T>
where
    T: Read + Seek,
{
    fs: T,
    ntfs: &'n Ntfs,
}


struct MFTShortEntry {
    record_number: u64,
    path:String,
    length: u64,
}

fn main() -> Result<()>{

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: ntfs-util drive");
        eprintln!();
        eprintln!("Drives can be a path to any NTFS filesystem image.");
        eprintln!("Under Windows and when run with administrative privileges, FILESYSTEM can also");
        eprintln!("be the special path \\\\.\\C: to access the filesystem of the C: partition.");
        bail!("Aborted");
    }
    let mut fs=open_drive("\\\\.\\D:")?;
    let mut ntfs = Ntfs::new(&mut fs).unwrap();

    
    

    /*
    for (key,value) in load_short_mft(&"./mft_dump"){
        println!("{}-{}#{}#{}",key,value.record_number,value.path,value.length);
    }
    */
   
    Ok(())

}

fn open_drive(device:&str) -> Result<BufReader<SectorReader<File>>>
{
    Ok(BufReader::new(SectorReader::new(File::open(device)?, 4096)?))

}

fn getNtfsInfo(ntfs:Ntfs) -> Result<()>{

    Ok(())
}

fn load_short_mft(path:&str) -> BTreeMap<u64,MFTShortEntry>{

    let mut mft_lite_entries:BTreeMap<u64,MFTShortEntry>=BTreeMap::new();

    //open file in read-only
    if let Ok(lines) = read_lines(path) {
        // Consumes the iterator, returns an (Optional) String
        for line in lines {
            if let Ok(l) = line {
               let vec: Vec<&str> = l.split('|').collect();
               let mft_short_entry=MFTShortEntry{
                record_number:vec[0].to_string().parse::<u64>().unwrap(),
                path:vec[1].trim().to_string(),
                length:vec[2].parse().unwrap(),
               };

               mft_lite_entries.insert(mft_short_entry.record_number, mft_short_entry);
               
            }
        }
    }
    
    mft_lite_entries
}


/**
 * Read the file filename line by line
 */
fn read_lines<P>(filename: P) -> Result<Lines<BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines())
}