
mod sector_reader;
use std::env;
use std::collections::{HashMap, BTreeMap};
use std::fmt::format;
use std::fs::{File, OpenOptions, create_dir, create_dir_all};
use std::io::{self, prelude::*, BufReader, Read, Seek, Write, Lines, BufWriter};
use std::num::NonZeroU64;

use anyhow::{anyhow, bail, Context, Result};
use ntfs::indexes::NtfsFileNameIndex;
use ntfs::structured_values::{
    NtfsAttributeList, NtfsFileName, NtfsFileNamespace, NtfsStandardInformation,
};
use ntfs::{Ntfs, NtfsAttribute, NtfsAttributeType, NtfsFile, NtfsReadSeek, NtfsError};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

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

#[derive(Serialize, Deserialize, Debug)]
struct RecoverableFile {
    id: String,
    file_name: String,
    size: u64,
    path: String,
    valid: bool
}

struct NtfsFolderName<'n> {
    ntfs_file: NtfsFile<'n>,
    ntfs_file_name: String
}

struct CommandInfo<'n, T>
where
    T: Read + Seek,
{
    fs: T,
    ntfs: &'n Ntfs,
}


struct MFTLiteEntry {
    is_deleted: bool,
    is_dir:bool,
    parent_entry: String,
    file_name: String,
}

struct MFTShortEntry {
    record_number: u64,
    path:String,
    length: u64,
}

fn main() -> Result<()>{

    let max_disk_size_mb=200000000000; //200GB
    //let max_disk_size_mb=20000000; //200GB

    let args: Vec<String> = env::args().collect();
    //let args=vec!["","24","22088"];

    let f = File::open("\\\\.\\E:")?;
    let sr = SectorReader::new(f, 4096)?;
    let mut fs = BufReader::new(sr);
    
    let mut ntfs = Ntfs::new(&mut fs).unwrap();

    let mut info = CommandInfo {
        fs,
        ntfs: &ntfs,
    };
    

    //repair_mft_from_range(&mut info, args[1].parse().unwrap(), args[2].parse().unwrap(),"revovery_dir".to_string());
    let mut total_size=0;
    let mut number_of_recovered_files=0;
    for (key,value) in load_short_mft(&"./mft_dump_1610"){
        //println!("{}-{}#{}#{}",key,value.record_number,value.path,value.length);
        total_size +=value.length;

        //workaround to be replaced with proper solution
    let path_splits:Vec<&str>=value.path.split("/").collect();
    let mut output_folder= String::from("./recovery_files_folder");
    let mut file_name=String::from("");

    if path_splits.len()>1 {
        output_folder.push_str("/");
        output_folder.push_str(&path_splits[0]);
        file_name.push_str(&path_splits[1]);
    } else {file_name.push_str(&value.path);}

    match copy_file(&mut info, value.record_number, &output_folder, &file_name){
            Ok(o)=> {},
            Err(e)=> {eprintln!("Error while attempting to copy the file: {}",e);
            continue}
        };

        number_of_recovered_files +=1;

        if total_size>max_disk_size_mb {break;}
    }

    let mega_bytes=total_size/1024/1024;

    println!("Recovery summary\n - Number of recovered files: {}\n - Total revocered size: {}",
        number_of_recovered_files, mega_bytes);

    Ok(())

}

/**
 * Try to repair a corrupted $MFT table
 */
fn repair_mft<T>(info:&mut CommandInfo<T>, o_folder:String)->Result<()>
where
T: Read + Seek,
{
    Ok(())
}


/**
 * Try to repair the $Mft from a start..end record range and save the valid records in an external file
 */
fn repair_mft_from_range<T>(info:&mut CommandInfo<T>, start: u64, end:u64, o_folder:String) -> Result<()>
  where
    T: Read + Seek,
    {

    let mut cache=HashMap::new();

    let mut o_path= String::new();

    let mut tmp_file_length:u64=0;

    let mft_dump_file_name= "./mft_dump"; //create the file in the local directory

    
    let mft_dump_file_formatted=format!("{}-{}-{}",mft_dump_file_name,start.to_string(),end.to_string());

    //Open file to write up the valid records
    let is_exist=Path::new(&mft_dump_file_formatted).exists();
    let mut mft_dump_file = OpenOptions::new()
    .create_new(!is_exist) 
    .write(true)
    .append(true)
    .open(&mft_dump_file_formatted)?;

    println!("File Mft dump {} created!",&mft_dump_file_formatted);
     
    
    for n in start..end {

        let file=match info.ntfs.file(&mut info.fs, n) {
            Ok(o)=> o,
            Err(e) => {
                eprintln!("Error while attempting to read the record {} from $MFT!...{}",n,e);
                continue},
            };
        if file.is_directory() {continue;}
        if let Ok(file_info)=get_file_info(&mut info.fs, &file){
            tmp_file_length=file_info.length;
            if file_info.file_name == "$MFT" {
                eprintln!("MFT record overflow!");
                break;}
                else if file_info.file_name!="" {
                        if let Some(parent_directory)=
                            get_directory(info, file_info.record_parent_number, &mut cache){
                                o_path=format!("./{}/{}/{}",&o_folder, parent_directory.file_name,file_info.file_name);
                        } else {
                                o_path=format!("./{}/{}/{}",&o_folder,&"Lost&Found",file_info.file_name);
                            }
                        }
           }
           //println!("{}#{}",n,o_path);

           let line=format!("{}|{}|{}\n",
                            n,
                            o_path,
                            tmp_file_length
                            );
          let mut buffer = BufWriter::new(&mft_dump_file);
          
         match  buffer.write(line.as_bytes()){
            Ok(o)=> {},
            Err(e) => {continue},
          };
        

          print!(".");
    }

    Ok(())
}



fn get_directory<'a,T>(info:&mut CommandInfo<T>, file_record_number: u64, cache:&'a mut HashMap<u64,FileInfo>)->Option<&'a FileInfo>
    where
    T: Read + Seek,
    {

        if !cache.contains_key(&file_record_number) {
            //println!("Cache missed! {}",file_record_number);
            if let Ok(file)=info.ntfs.file(&mut info.fs, file_record_number){
                if let Ok(_file_info)=get_file_info(&mut info.fs, &file){
                    cache.insert(file.file_record_number(),_file_info);
                }
        }
    }
        cache.get(&file_record_number)
        
}

/**
 * Return the main info for a given file as input
 */
fn get_file_info<T>(fs: &mut T, file:& NtfsFile) -> Result<FileInfo,NtfsError>
    where
    T: Read + Seek,
{
    const TIME_FORMAT: &[FormatItem] =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second] UTC");

    let mut file_info=FileInfo{
        record_number: 0,
        record_parent_number: 0,
        file_name: String::new(),
        length: 0,
        path: String::new(),
        access_time:String::new(),
        creation_time:String::new(),
        modification_time:String::new()
    };
   
    let mut attributes = file.attributes();
    while let Some(attribute_item) = attributes.next(fs) {
        let attribute_item = attribute_item?;
        let attribute = attribute_item.to_attribute();

        match attribute.ty() {
            Ok(NtfsAttributeType::StandardInformation) => {
                let std_info = attribute.resident_structured_value::<NtfsStandardInformation>()?;
                if let Ok(atime)=OffsetDateTime::from(std_info.access_time()).format(TIME_FORMAT){
                    file_info.access_time=atime;
                }
                if let Ok(mtime)=OffsetDateTime::from(std_info.modification_time()).format(TIME_FORMAT){
                    file_info.modification_time=mtime;
                }
                if let Ok(ctime)=OffsetDateTime::from(std_info.creation_time()).format(TIME_FORMAT){
                    file_info.creation_time=ctime;
                }
                
            },
            Ok(NtfsAttributeType::FileName) => {
                let file_name = attribute.structured_value::<_, NtfsFileName>(fs)?;
                file_info.file_name=file_name.name().to_string_lossy();
                file_info.record_parent_number=file_name.parent_directory_reference().file_record_number();
            },
            Ok(NtfsAttributeType::Data) => {
                file_info.length=attribute.value_length();
                file_info.record_number=file.file_record_number();
                
            },
            _ => continue,
        }
    }

    Ok(file_info)

}


fn load_short_mft(path:&str) -> BTreeMap<u64,MFTShortEntry>{

    let mut mft_lite_entries:BTreeMap<u64,MFTShortEntry>=BTreeMap::new();

    //open file in read-only
    if let Ok(lines) = read_lines(path) {
        // Consumes the iterator, returns an (Optional) String
        println!("Read line by line the file {}",path);
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

    println!("Summary BTree\n -Total record number:{}",mft_lite_entries.len());
    
    mft_lite_entries
}

fn load_simplified_mft_entries(path:&str) -> HashMap<String,MFTLiteEntry>{

    let mut mft_lite_entries:HashMap<String,MFTLiteEntry>=HashMap::new();

    //open file in read-only
    if let Ok(lines) = read_lines(path) {
        // Consumes the iterator, returns an (Optional) String
        for line in lines {
            if let Ok(l) = line {
               // println!("{}", &l);
             //  let split= &l.split('|');
               let vec: Vec<&str> = l.split('|').collect();

               //println!("VECTOR::{}",vec[0]);

               let mut _is_deleted=false;
               let mut _is_dir=true;
               
               if vec[1]=="0" {
                _is_deleted== true;
               }
               if vec[2]=="0" {
                _is_dir=false;
               }
               let mft_lite_entry=MFTLiteEntry{
                is_deleted:_is_deleted,
                is_dir:_is_dir,
                parent_entry:vec[3].trim().to_string(),
                file_name:vec[4].to_string(),
               };
               
               mft_lite_entries.insert(vec[0].trim().trim_start().trim_end().to_string(), mft_lite_entry);
                //println!("{}",s);
               
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


fn copy_file<T> (info: &mut CommandInfo<T>, record_number: u64, out_folder_path: &str, file_name: &str) -> Result<()>
 where
     T: Read + Seek,
 {
    
    //open the file that copies the target content to

    if !Path::new(&out_folder_path).exists() {
        create_dir_all(&out_folder_path)?;
    }
    
    // handle just one level folder for now
    let mut file_path=format!("{}/{}",out_folder_path,file_name);
    

    let mut output_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file_path)
        .with_context(|| format!("Tried to open \"{file_path}\" for writing"))?;
    
    let _file=info.ntfs.file(&mut info.fs, record_number)?;

    let data_stream_name="";

    if let Some(data_item)= _file.data(&mut info.fs, data_stream_name) {
 
    let data_item = data_item?;

    let data_attribute = data_item.to_attribute();

    let mut data_value = data_attribute.value(&mut info.fs)?;

    let mut buf = [0u8; 4096];

    print!(
        "Mft record #{}# - saved {} bytes of data in \"{}\"...",
        record_number,
        data_value.len(),
        file_path
    );

    loop {
        let bytes_read_result = data_value.read(&mut info.fs, &mut buf);
       
        let bytes_read = match bytes_read_result {
            Ok(x)=>x,
            Err(error) => {break;},
        };

        if bytes_read == 0 {
            break;
        }

        output_file.write(&buf[..bytes_read])?;
    }
} else {
    println!(
        "The file does not have a \"{}\" $DATA attribute.",
        data_stream_name
    );
}
    println!("[OK]");

    Ok(())
 }

/**
 * Browse NTFS registry and copy recoverable files and save the list to a json file.
 * 
 */
fn ntfs_files_browse_and_save<T> (info: &mut CommandInfo<T>, root: NtfsFile, dry_run:bool) -> Result<()>
 where
     T: Read + Seek,
 {

    let mut ntfs_file_name_list:Vec<NtfsFolderName>=Vec::new();
    let mut file_stack: Vec<NtfsFileName> = Vec::new();
    //let root_directory: NtfsFile=info.current_directory;
    let mut file_list: Vec<RecoverableFile> = Vec::new();
    let mut current_directory_option;

    let recovery_folder=String::from("recovery_folder_new");

    let data_stream_name="";

    let mut ntfs_folder_name_entry= NtfsFolderName {
        ntfs_file:root,
        ntfs_file_name: String::from("ROOT")
    };

    ntfs_file_name_list.push(ntfs_folder_name_entry);
    
    'main: loop{
        println!("### Folder Stack size:{}",ntfs_file_name_list.len());
        println!("### File Stack size:{}",file_list.len());

        if ntfs_file_name_list.is_empty() /*|| file_list.len()>100*/ {
            println!("No futher file to get");
            break;
        }

        current_directory_option=ntfs_file_name_list.pop();

        let current_directory: NtfsFolderName= match  current_directory_option{
            Some(p) => p,
            None => continue,
        };

        let index_result = current_directory
                .ntfs_file
                .directory_index(&mut info.fs);
        
        let index=match index_result {
            Ok(x) => x,
            Err(e) => continue,
        };

        let mut iter = index.entries();
        
        while let Some(entry) = iter.next(&mut info.fs) {
            
            let entry_result = entry;

            let entry=match entry_result {
                Ok(x) => x,
                Err(error) => continue,
            };

            let file_name = entry
                .key()
                .expect("key must exist for a found Index Entry")?;
           
            if file_name.is_directory() && 
                file_name.name()!=".." && 
                file_name.name()!="." {
                let prefix="<DIR>";
                println!("{:5}  {}", prefix, file_name.name());

                let file_result = entry.to_file(info.ntfs, &mut info.fs);
                    
                let _file = match file_result {
                    Ok(file)=>file,
                    Err(error) => continue,
                };

                println!("### Push folder to stack!");
                let ntfs_folder_name_entry= NtfsFolderName {
                    ntfs_file: _file,
                    ntfs_file_name: file_name.name().to_string()
                };
                ntfs_file_name_list.push(ntfs_folder_name_entry);

            } else {
                println!("Got File {}", file_name.name());
                
                let recovered_file = RecoverableFile{
                    id:file_name.name().to_string(),
                    file_name:file_name.name().to_string(),
                    size:file_name.allocated_size(),
                    path:current_directory.ntfs_file_name.to_string(),
                    valid:true
                };

                 // read the file to copy
                 let file_result = entry.to_file(info.ntfs, &mut info.fs);
                    
                 let _file = match file_result {
                     Ok(file)=>file,
                     Err(error) => continue,
                 };

                 println!("HEADER DELETED FLAG:: {}",_file.flags().bits());

                if !dry_run{
                    let folder_path=format!("./{}/{}",
                     recovery_folder,recovered_file.path);

                    let _output_file_name=format!("{}/{}",
                     &folder_path,&recovered_file.file_name);

                    //create folders
                    let result_create_folder=create_dir_all(&folder_path);
                    if !std::path::Path::new(&folder_path).exists() {
                        let result_create_folder=create_dir_all(&folder_path);
                        let folder= match result_create_folder {
                            Ok(x)=>x,
                            Err(err)=> continue,
                        };
                        println!("Created folder:{}",folder_path);
                    }   
                   
                    let mut output_file_result = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&_output_file_name)
                    .with_context(|| format!("Tried to open \"{_output_file_name}\" for writing"));
                    let mut output_file = match output_file_result{
                        Ok(x)=> x,
                        Err(error) => {continue;}
                    };

                
                    let data_item = match _file.data(&mut info.fs, data_stream_name) {
                        Some(data_item) => data_item,
                        None => {
                            println!(
                                "The file does not have a \"{}\" $DATA attribute.",
                                data_stream_name
                            );
                            continue;
                        }
                    };
                    let data_item_result = data_item;

                    let data_item= match data_item_result {
                        Ok(x) => x,
                        Err(error)=> continue,
                    };

                    let data_attribute = data_item.to_attribute();

                    let mut data_value_result = data_attribute.value(&mut info.fs);

                    let mut data_value= match data_value_result {
                        Ok(x) => x,
                        Err(error)=> continue,
                    };
                
                    println!(
                        "Saving {} bytes of data in \"{}\"...",
                        data_value.len(),
                        _output_file_name
                    );
                    let mut buf = [0u8; 4096];
                
                    loop {
                        let bytes_read_result = data_value.read(&mut info.fs, &mut buf);
                        let bytes_read = match bytes_read_result {
                            Ok(x)=>x,
                            Err(error) => {break;},
                        };

                        if bytes_read == 0 {
                            break;
                        }
                
                        output_file.write(&buf[..bytes_read])?;
                    }
                    println!("### File {} restored!",_output_file_name);
                }

                file_list.push(recovered_file);

       }
            //file_stack.push(file_name);
        }
    }

     //Save the file list to external json file
     let serialized = serde_json::to_string(&file_list).unwrap();
     let mut file = File::create("ntfs.json")?;
        file.write_all(serialized.as_bytes())?;


    Ok(())

}

fn fileinfo_std(attribute: NtfsAttribute) -> Result<()> {
    const TIME_FORMAT: &[FormatItem] =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second] UTC");

    println!();
    println!("{:=^72}", " STANDARD INFORMATION ");

    let std_info = attribute.resident_structured_value::<NtfsStandardInformation>()?;

    println!("{:34}{:?}", "Attributes:", std_info.file_attributes());

    let atime = OffsetDateTime::from(std_info.access_time())
        .format(TIME_FORMAT)
        .unwrap();
    let ctime = OffsetDateTime::from(std_info.creation_time())
        .format(TIME_FORMAT)
        .unwrap();
    let mtime = OffsetDateTime::from(std_info.modification_time())
        .format(TIME_FORMAT)
        .unwrap();
    let mmtime = OffsetDateTime::from(std_info.mft_record_modification_time())
        .format(TIME_FORMAT)
        .unwrap();

    println!("{:34}{}", "Access Time:", atime);
    println!("{:34}{}", "Creation Time:", ctime);
    println!("{:34}{}", "Modification Time:", mtime);
    println!("{:34}{}", "MFT Record Modification Time:", mmtime);

    // NTFS 3.x extended information
    let class_id = std_info
        .class_id()
        .map(|x| x.to_string())
        .unwrap_or_else(|| "<NONE>".to_string());
    let maximum_versions = std_info
        .maximum_versions()
        .map(|x| x.to_string())
        .unwrap_or_else(|| "<NONE>".to_string());
    let owner_id = std_info
        .owner_id()
        .map(|x| x.to_string())
        .unwrap_or_else(|| "<NONE>".to_string());
    let quota_charged = std_info
        .quota_charged()
        .map(|x| x.to_string())
        .unwrap_or_else(|| "<NONE>".to_string());
    let security_id = std_info
        .security_id()
        .map(|x| x.to_string())
        .unwrap_or_else(|| "<NONE>".to_string());
    let usn = std_info
        .usn()
        .map(|x| x.to_string())
        .unwrap_or_else(|| "<NONE>".to_string());
    let version = std_info
        .version()
        .map(|x| x.to_string())
        .unwrap_or_else(|| "<NONE>".to_string());
    println!("{:34}{}", "Class ID:", class_id);
    println!("{:34}{}", "Maximum Versions:", maximum_versions);
    println!("{:34}{}", "Owner ID:", owner_id);
    println!("{:34}{}", "Quota Charged:", quota_charged);
    println!("{:34}{}", "Security ID:", security_id);
    println!("{:34}{}", "USN:", usn);
    println!("{:34}{}", "Version:", version);

    Ok(())
}

fn fileinfo_filename<T>(info: &mut CommandInfo<T>, attribute: NtfsAttribute) -> Result<()>
where
    T: Read + Seek,
{
    println!();
    println!("{:=^72}", " FILE NAME ");

    let file_name = attribute.structured_value::<_, NtfsFileName>(&mut info.fs)?;

    println!("{:34}\"{}\"", "Name:", file_name.name().to_string_lossy());
    println!("{:34}{:?}", "Namespace:", file_name.namespace());
    println!(
        "{:34}{:#x}",
        "Parent Directory Record Number:",
        file_name.parent_directory_reference().file_record_number()
    );

    Ok(())
}

fn fileinfo_data(attribute: NtfsAttribute) -> Result<()> {
    println!();
    println!("{:=^72}", " DATA STREAM ");

    println!("{:34}\"{}\"", "Name:", attribute.name()?.to_string_lossy());
    println!("{:34}{}", "Size:", attribute.value_length());

    Ok(())
}

fn fsinfo<T>(info: &mut CommandInfo<T>) -> Result<()>
where
    T: Read + Seek,
{
    println!("{:20}{}", "Cluster Size:", info.ntfs.cluster_size());
    println!("{:20}{}", "File Record Size:", info.ntfs.file_record_size());
    println!("{:20}{:#x}", "MFT Byte Position:", info.ntfs.mft_position());

    let volume_info = info.ntfs.volume_info(&mut info.fs)?;
    let ntfs_version = format!(
        "{}.{}",
        volume_info.major_version(),
        volume_info.minor_version()
    );
    println!("{:20}{}", "NTFS Version:", ntfs_version);

    println!("{:20}{}", "Sector Size:", info.ntfs.sector_size());
    println!("{:20}{}", "Serial Number:", info.ntfs.serial_number());
    println!("{:20}{}", "Size:", info.ntfs.size());

    let volume_name = if let Some(Ok(volume_name)) = info.ntfs.volume_name(&mut info.fs) {
        format!("\"{}\"", volume_name.name())
    } else {
        "<NONE>".to_string()
    };
    println!("{:20}{}", "Volume Name:", volume_name);

    Ok(())
}