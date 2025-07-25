pub const TARGET_ZIP_VERSION: u16 = 45;

pub const GENERAL_PURPOSE_BIT_FLAG_DEFLATE_NORMAL: u16 = 0;
pub const COMPRESSION_METHOD_DEFLATE: u16 = 8;


#[repr(C)]
pub struct LocalFileHeader {
    pub signature: [u8; 4],
    pub version_needed_to_extract: u16,
    pub general_purpose_bit_flag: u16,
    pub compression_method: u16,
    pub last_modified_file_time: u16,
    pub last_modified_file_date: u16,
    pub crc_32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub file_name_length: u16,
    pub extra_field_length: u16,
}
impl LocalFileHeader {
    pub const SIGNATURE: [u8; 4] = [b'P', b'K', 3, 4];

    pub fn to_bytes(&self) -> [u8; 30] {
        let mut bytes = [0u8; 30];
        bytes[00..04].copy_from_slice(&self.signature);
        bytes[04..06].copy_from_slice(&self.version_needed_to_extract.to_le_bytes());
        bytes[06..08].copy_from_slice(&self.general_purpose_bit_flag.to_le_bytes());
        bytes[08..10].copy_from_slice(&self.compression_method.to_le_bytes());
        bytes[10..12].copy_from_slice(&self.last_modified_file_time.to_le_bytes());
        bytes[12..14].copy_from_slice(&self.last_modified_file_date.to_le_bytes());
        bytes[14..18].copy_from_slice(&self.crc_32.to_le_bytes());
        bytes[18..22].copy_from_slice(&self.compressed_size.to_le_bytes());
        bytes[22..26].copy_from_slice(&self.uncompressed_size.to_le_bytes());
        bytes[26..28].copy_from_slice(&self.file_name_length.to_le_bytes());
        bytes[28..30].copy_from_slice(&self.extra_field_length.to_le_bytes());
        bytes
    }
}

#[repr(C)]
pub struct CentralDirectoryHeader {
    pub signature: [u8; 4],
    pub version_made_by: u16,
    pub version_needed_to_extract: u16,
    pub general_purpose_bit_flag: u16,
    pub compression_method: u16,
    pub last_modified_file_time: u16,
    pub last_modified_file_date: u16,
    pub crc_32: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub file_name_length: u16,
    pub extra_field_length: u16,
    pub file_comment_length: u16,
    pub disk_number_start: u16,
    pub internal_file_attributes: u16,
    pub external_file_attributes: u16,
    pub relative_offset_of_local_header: u32,
}
impl CentralDirectoryHeader {
    pub const SIGNATURE: [u8; 4] = [b'P', b'K', 1, 2];

    pub fn to_bytes(&self) -> [u8; 44] {
        let mut bytes = [0u8; 44];
        bytes[00..04].copy_from_slice(&self.signature);
        bytes[04..06].copy_from_slice(&self.version_made_by.to_le_bytes());
        bytes[06..08].copy_from_slice(&self.version_needed_to_extract.to_le_bytes());
        bytes[08..10].copy_from_slice(&self.general_purpose_bit_flag.to_le_bytes());
        bytes[10..12].copy_from_slice(&self.compression_method.to_le_bytes());
        bytes[12..14].copy_from_slice(&self.last_modified_file_time.to_le_bytes());
        bytes[14..16].copy_from_slice(&self.last_modified_file_date.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.crc_32.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.compressed_size.to_le_bytes());
        bytes[24..28].copy_from_slice(&self.uncompressed_size.to_le_bytes());
        bytes[28..30].copy_from_slice(&self.file_comment_length.to_le_bytes());
        bytes[30..32].copy_from_slice(&self.extra_field_length.to_le_bytes());
        bytes[32..34].copy_from_slice(&self.file_comment_length.to_le_bytes());
        bytes[34..36].copy_from_slice(&self.disk_number_start.to_le_bytes());
        bytes[36..38].copy_from_slice(&self.internal_file_attributes.to_le_bytes());
        bytes[38..40].copy_from_slice(&self.external_file_attributes.to_le_bytes());
        bytes[40..44].copy_from_slice(&self.relative_offset_of_local_header.to_le_bytes());
        bytes
    }
}

#[repr(C)]
pub struct Zip64ExtraField {
    pub header_id: u16,
    pub data_size: u16,
    pub uncompressed_size: u64,
    pub compressed_size: u64,
    pub relative_offset_of_local_header: u64,
    pub disk_number_start: u32,
}
impl Zip64ExtraField {
    pub const HEADER_ID: u16 = 1;

    pub fn select_to_bytes(&self, selected: &[Zip64ExtraFieldSelect]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32);
        let mut size: u16 = 0;
        bytes.extend_from_slice(&self.header_id.to_le_bytes());
        bytes.extend_from_slice(&size.to_le_bytes());
        if selected.contains(&Zip64ExtraFieldSelect::UncompressedSize) {
            size += size_of::<u64>() as u16;
            bytes.extend_from_slice(&self.uncompressed_size.to_le_bytes());
        }
        if selected.contains(&Zip64ExtraFieldSelect::CompressedSize) {
            size += size_of::<u64>() as u16;
            bytes.extend_from_slice(&self.compressed_size.to_le_bytes());
        }
        if selected.contains(&Zip64ExtraFieldSelect::RelativeOffsetOfLocalHeader) {
            size += size_of::<u64>() as u16;
            bytes.extend_from_slice(&self.relative_offset_of_local_header.to_le_bytes());
        }
        if selected.contains(&Zip64ExtraFieldSelect::DiskNumberStart) {
            size += size_of::<u32>() as u16;
            bytes.extend_from_slice(&self.disk_number_start.to_le_bytes());
        }
        bytes[2..4].copy_from_slice(&size.to_le_bytes());
        bytes
    }
}

#[derive(PartialEq)]
pub enum Zip64ExtraFieldSelect {
    UncompressedSize,
    CompressedSize,
    RelativeOffsetOfLocalHeader,
    DiskNumberStart,
}

#[repr(C)]
pub struct Zip64EndOfCentralDirectoryRecord {
    pub signature: [u8; 4],
    pub size_of_zip64_end_of_central_directory_record: u64,
    pub version_made_by: u16,
    pub version_needed_to_extract: u16,
    pub number_of_this_disk: u32,
    pub number_of_the_disk_with_the_start_of_the_central_directory: u32,
    pub total_number_of_entries_in_the_central_directory_on_this_disk: u64,
    pub total_number_of_entries_in_the_central_directory: u64,
    pub size_of_the_central_directory: u64,
    pub offset_of_start_of_central_directory_with_respect_to_the_starting_disk_number: u64,
    //zip64_extensible_data_sector
}
impl Zip64EndOfCentralDirectoryRecord {
    pub const SIGNATURE: [u8; 4] = [b'P', b'K', 6, 6];

    pub fn to_bytes(&self) -> [u8; 56] {
        let mut bytes = [0u8; 56];
        bytes[00..04].copy_from_slice(&self.signature);
        bytes[04..12].copy_from_slice(&self.size_of_zip64_end_of_central_directory_record.to_le_bytes());
        bytes[12..14].copy_from_slice(&self.version_made_by.to_le_bytes());
        bytes[14..16].copy_from_slice(&self.version_needed_to_extract.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.number_of_this_disk.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.number_of_the_disk_with_the_start_of_the_central_directory.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.total_number_of_entries_in_the_central_directory_on_this_disk.to_le_bytes());
        bytes[32..40].copy_from_slice(&self.total_number_of_entries_in_the_central_directory.to_le_bytes());
        bytes[40..48].copy_from_slice(&self.size_of_the_central_directory.to_le_bytes());
        bytes[48..56].copy_from_slice(&self.offset_of_start_of_central_directory_with_respect_to_the_starting_disk_number.to_le_bytes());
        bytes
    }
}

#[repr(C)]
pub struct Zip64EndOfCentralDirectoryLocator {
    pub signature: [u8; 4],
    pub number_of_the_disk_with_the_start_of_the_zip64_end_of_central_irectory: u32,
    pub relative_offset_of_the_zip64_end_of_central_directory_record: u64,
    pub total_number_of_disks: u32,
}
impl Zip64EndOfCentralDirectoryLocator {
    pub const SIGNATURE: [u8; 4] = [b'P', b'K', 6, 7];

    pub fn to_bytes(&self) -> [u8; 20] {
        let mut bytes = [0u8; 20];
        bytes[00..04].copy_from_slice(&self.signature);
        bytes[04..08].copy_from_slice(&self.number_of_the_disk_with_the_start_of_the_zip64_end_of_central_irectory.to_le_bytes());
        bytes[08..16].copy_from_slice(&self.relative_offset_of_the_zip64_end_of_central_directory_record.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.total_number_of_disks.to_le_bytes());
        bytes
    }
}

#[repr(C)]
pub struct EndOfCentralDirectoryRecord {
    pub signature: [u8; 4],
    pub number_of_this_disk: u16,
    pub number_of_the_disk_with_the_start_of_the_central_directory: u16,
    pub total_number_of_entries_in_the_central_directory_on_this_disk: u16,
    pub total_number_of_entries_in_the_central_directory: u16,
    pub size_of_the_central_directory: u32,
    pub offset_of_start_of_central_directory_with_respect_to_the_starting_disk_number: u32,
    pub zip_file_comment_length: u16,
}
impl EndOfCentralDirectoryRecord {
    pub const SIGNATURE: [u8; 4] = [b'P', b'K', 5, 6];

    pub fn to_bytes(&self) -> [u8; 22] {
        let mut bytes = [0u8; 22];
        bytes[00..04].copy_from_slice(&self.signature);
        bytes[04..06].copy_from_slice(&self.number_of_this_disk.to_le_bytes());
        bytes[06..08].copy_from_slice(&self.number_of_the_disk_with_the_start_of_the_central_directory.to_le_bytes());
        bytes[08..10].copy_from_slice(&self.total_number_of_entries_in_the_central_directory_on_this_disk.to_le_bytes());
        bytes[10..12].copy_from_slice(&self.total_number_of_entries_in_the_central_directory.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.size_of_the_central_directory.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.offset_of_start_of_central_directory_with_respect_to_the_starting_disk_number.to_le_bytes());
        bytes[20..22].copy_from_slice(&self.zip_file_comment_length.to_le_bytes());
        bytes
    }
}
