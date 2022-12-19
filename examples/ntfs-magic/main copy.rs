
mod sector_reader;
use std::collections::HashMap;
use std::fmt::format;
use std::fs::{File, OpenOptions, create_dir, create_dir_all};
use std::io::{self, prelude::*, BufReader, Read, Seek, Write, Lines};
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


fn main() -> Result<()>{

    let f = File::open("\\\\.\\E:")?;
    let sr = SectorReader::new(f, 4096)?;
    let mut fs = BufReader::new(sr);
    
    let mut ntfs = Ntfs::new(&mut fs).unwrap();
/*
    let info_result=ntfs.volume_info(&mut fs);
    let info= match info_result {
        Ok(o)=>o,
        Err(e)=>panic!("{}",e)
    };
 */
    let mut info = CommandInfo {
        fs,
        ntfs: &ntfs,
    };

    println!("NTFS DISK SIZE: {}",ntfs.size());
    println!("NTFS DISK SIZE: {}",ntfs.file_record_size());
   
    //let root_dir = ntfs.root_directory(&mut fs).unwrap();
    
   let mut record_number=0;

   if let Ok(file)=ntfs.file(&mut info.fs, 100){
    if let Ok(file_info)=get_file_info(&mut info, &file){
        if file_info.file_name!="" {
            println!("{:?}",file_info);
        }
   }
}

   /*
   loop {

    if let Ok(file)=ntfs.file(&mut info.fs, 12){
        if let Ok(file_info)=get_file_info(&mut info, &file){
            if file_info.file_name!="" {
                if file_info.record_parent_number>0 {
                    if let Ok(file_parent)=ntfs.file(&mut info.fs, file_info.record_parent_number){
                       
                    }
                }
            } 
            
            else {

            }
        }

    } else {
        continue;
    }
    */


    /*
    let mut attributes = file.attributes();
    while let Some(attribute_item) = attributes.next(&mut info.fs) {
        let attribute_item = attribute_item?;
        let attribute = attribute_item.to_attribute();

        match attribute.ty() {
            Ok(NtfsAttributeType::StandardInformation) => fileinfo_std(attribute)?,
            Ok(NtfsAttributeType::FileName) => fileinfo_filename(&mut info, attribute)?,
            Ok(NtfsAttributeType::Data) => fileinfo_data(attribute)?,
            _ => continue,
        }
    }
    */


//    println!("{}",file.flags().bits());
//    println!("{}",file.is_directory());
//    println!("{}",file.allocated_size());
//    println!("{}",file.data_size());

   


   // ntfs_files_browse_and_save(&mut info,file,false);


/*
    while let Some(entry)= file.attributes().next(&mut fs) {
        
        println!("loopppp");
        match(entry) {
            Ok(o) => {

                println!("{}",o.to_attribute().name().unwrap().to_string());

            },
            Err(e) => {
                break;
            }
        }
        
    }
 */
    
    //let current_directory = vec![&root_dir];
    //let _file=ntfs.file(&mut fs, 20579).unwrap();
    
    //francesco 077.jpg
    //recovery_folder_new
    //183
    
    //println!("{}",_file.flags().bits());

/*

    let mut info = CommandInfo {
        fs,
        ntfs: &ntfs,
    };

    let mft_lite_entries=load_simplified_mft_entries("./mft_dump");

    for (key,value) in mft_lite_entries.iter(){

        println!("'{}'",key);

        //let record_number=key.trim_a .trim_matches(char::from(0));

        let u64_record_number:u64=key.parse()?;
       // let parent_number=&value.parent_entry;

        let mut output_dir_path= String::new();

        if let Some(e)=mft_lite_entries.get(&value.parent_entry){
            output_dir_path=String::from(&e.file_name);
        } else {
            output_dir_path=String::from("Lost&File")
        }

        let mut output_path=format!("{}/{}","./recovery_folder_new",output_dir_path);

        if let Some(e)=
        copy_file(&mut info, u64_record_number as u64,
           &output_path , &value.file_name).err(){
        println!("Error:{}",e);
        continue;
    }

    }
    
    println!("Continue...");
*/
    Ok(())

}

/**
 * Return the main info for a given file as input
 */
fn get_file_info<T>(info: &mut CommandInfo<T>, file:& NtfsFile) -> Result<FileInfo,NtfsError>
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
    while let Some(attribute_item) = attributes.next(&mut info.fs) {
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
                let file_name = attribute.structured_value::<_, NtfsFileName>(&mut info.fs)?;
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

    if !Path::new(out_folder_path).exists() {
        let result_create_folder=create_dir_all(out_folder_path)?;
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

    println!(
        "Saving {} bytes of data in \"{}\"...",
        data_value.len(),
        file_path
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
} else {
    println!(
        "The file does not have a \"{}\" $DATA attribute.",
        data_stream_name
    );
}
    println!("### File {} restored!",file_path);

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