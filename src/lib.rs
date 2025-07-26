use std::{
    ffi::OsString,
    fs::File,
    io::{
        self,
        ErrorKind,
        Seek,
        SeekFrom,
        Write,
    },
    path::Path,
};
use flate2::{
    read::DeflateDecoder,
    write::DeflateEncoder,
    Compression,
};

mod common;
mod crc32;

use crate::common::{
    CentralDirectoryHeader,
    EndOfCentralDirectoryRecord,
    LocalFileHeader,
    Zip64EndOfCentralDirectoryLocator,
    Zip64EndOfCentralDirectoryRecord,
    Zip64ExtraField,
    Zip64ExtraFieldSelect,
    COMPRESSION_METHOD_DEFLATE,
    GENERAL_PURPOSE_BIT_FLAG_DEFLATE_NORMAL,
    TARGET_ZIP_VERSION,
};


pub struct Writer {
    crc_32: u32,
    local_file_name: OsString,
    deflate_writer: DeflateEncoder<File>,
}
impl Writer {
    pub fn create(path: &Path) -> io::Result<Self> {
        let local_file_name = match path.file_stem() {
            None => return Err(io::Error::new(ErrorKind::InvalidFilename, "no file name")),
            Some(s) => if s.len() > u16::MAX as usize {
                return Err(io::Error::new(ErrorKind::InvalidFilename, "file name too long"));
            } else {
                s.to_os_string()
            },
        };
        let mut file = std::fs::File::create(path)?;

        // write local file header
        let zip64_extra_field = Zip64ExtraField {
            header_id: Zip64ExtraField::HEADER_ID,
            data_size: 0, // ignore, will auto set in `select_to_bytes`
            uncompressed_size: 0, // placeholder
            compressed_size: 0, // placeholder
            relative_offset_of_local_header: 0, // irrelevant in local file header
            disk_number_start: 0, // irrelevant in local file header
        }.select_to_bytes(&[
            Zip64ExtraFieldSelect::UncompressedSize,
            Zip64ExtraFieldSelect::CompressedSize,
        ]);
        let local_file_header = LocalFileHeader {
            signature: LocalFileHeader::SIGNATURE,
            version_needed_to_extract: TARGET_ZIP_VERSION,
            general_purpose_bit_flag: GENERAL_PURPOSE_BIT_FLAG_DEFLATE_NORMAL,
            compression_method: COMPRESSION_METHOD_DEFLATE,
            last_modified_file_time: 0, // nobody cares
            last_modified_file_date: 0, // nobody cares
            crc_32: 0, // placeholder
            compressed_size: u32::MAX, // actual value is stored in zip64 extra field
            uncompressed_size: u32::MAX, // actual value is stored in zip64 extra field
            file_name_length: local_file_name.len() as u16,
            extra_field_length: zip64_extra_field.len() as u16,
        }.to_bytes();
        file.write_all(&local_file_header)?;
        file.write_all(&local_file_name.as_encoded_bytes())?;
        file.write_all(&zip64_extra_field)?;

        Ok(Self {
            crc_32: 0,
            local_file_name,
            deflate_writer: DeflateEncoder::new(file, Compression::best()),
        })
    }

    pub fn finish(mut self) -> io::Result<File> {
        self.deflate_writer.flush()?;
        let uncompressed_size = self.deflate_writer.total_in();
        let compressed_size = self.deflate_writer.total_out() + 2; // will write 0x0003 when deflate close
        let mut file = self.deflate_writer.finish()?;

        // update local file header
        let cd_pos = file.stream_position()?;
            // update crc 32
            file.seek(SeekFrom::Start(14))?;
            file.write_all(&self.crc_32.to_le_bytes())?;
            // update uncompressed size
            file.seek(SeekFrom::Current(self.local_file_name.len() as i64 + 16))?;
            file.write_all(&uncompressed_size.to_le_bytes())?;
            // update compressed size
            file.write_all(&compressed_size.to_le_bytes())?;
        file.seek(SeekFrom::Start(cd_pos))?;

        // write central directory
        // write central directory header
        let use_zip64_cd_header =
            compressed_size >= u32::MAX as u64 ||
            uncompressed_size >= u32::MAX as u64 ||
            cd_pos >= u32::MAX as u64;
        let zip64_extra_field = if use_zip64_cd_header {
            Some(Zip64ExtraField {
                header_id: Zip64ExtraField::HEADER_ID,
                data_size: 0, // ignore, will auto set in `select_to_bytes`
                uncompressed_size,
                compressed_size,
                relative_offset_of_local_header: cd_pos as u64, // reverse relative offset to file start
                disk_number_start: 0, // no multiple volume
            }.select_to_bytes(&[
                Zip64ExtraFieldSelect::UncompressedSize,
                Zip64ExtraFieldSelect::CompressedSize,
                Zip64ExtraFieldSelect::RelativeOffsetOfLocalHeader,
            ]))
        } else { None };
        let central_directory_header = CentralDirectoryHeader {
            signature: CentralDirectoryHeader::SIGNATURE,
            version_made_by: TARGET_ZIP_VERSION,
            version_needed_to_extract: TARGET_ZIP_VERSION,
            general_purpose_bit_flag: GENERAL_PURPOSE_BIT_FLAG_DEFLATE_NORMAL,
            compression_method: COMPRESSION_METHOD_DEFLATE,
            last_modified_file_time: 0, // nobody cares
            last_modified_file_date: 0, // nobody cares
            crc_32: self.crc_32,
            compressed_size: if zip64_extra_field.is_none() { compressed_size as u32 } else { u32::MAX },
            uncompressed_size: if zip64_extra_field.is_none() { uncompressed_size as u32 } else { u32::MAX },
            file_name_length: self.local_file_name.len() as u16,
            extra_field_length: if let Some(z64) = zip64_extra_field.as_ref() { z64.len() as u16 } else { 0 },
            file_comment_length: 0, // no comment
            disk_number_start: 0, // no multiple volume
            internal_file_attributes: 0,
            external_file_attributes: 0,
            relative_offset_of_local_header: u32::MAX, // actual value is stored in zip64 extra field
        }.to_bytes();
        file.write_all(&central_directory_header)?;
        file.write_all(&self.local_file_name.as_encoded_bytes())?;
        if let Some(z64) = zip64_extra_field.as_ref() {
            file.write_all(&z64)?;
        }
        let cd_size = file.stream_position()? - cd_pos;

        // write end of central direction
        let use_zip64_ending = cd_pos >= u32::MAX as u64;
        if use_zip64_ending {
            // write zip64 end of central directory record
            let rec_pos = file.stream_position()?;
            let zip64_end_of_central_directory_record = Zip64EndOfCentralDirectoryRecord {
                signature: Zip64EndOfCentralDirectoryRecord::SIGNATURE,
                size_of_zip64_end_of_central_directory_record: 44,
                version_made_by: TARGET_ZIP_VERSION,
                version_needed_to_extract: TARGET_ZIP_VERSION,
                number_of_this_disk: 0,
                number_of_the_disk_with_the_start_of_the_central_directory: 0,
                total_number_of_entries_in_the_central_directory_on_this_disk: 1,
                total_number_of_entries_in_the_central_directory: 1,
                size_of_the_central_directory: cd_size,
                offset_of_start_of_central_directory_with_respect_to_the_starting_disk_number: cd_pos,
            }.to_bytes();
            file.write_all(&zip64_end_of_central_directory_record)?;
            // write zip64 end of central directory locator
            let zip64_end_of_central_directory_locator = Zip64EndOfCentralDirectoryLocator {
                signature: Zip64EndOfCentralDirectoryLocator::SIGNATURE,
                number_of_the_disk_with_the_start_of_the_zip64_end_of_central_irectory: 0,
                relative_offset_of_the_zip64_end_of_central_directory_record: rec_pos,
                total_number_of_disks: 1,
            }.to_bytes();
            file.write_all(&zip64_end_of_central_directory_locator)?;
        }

        // write end of central directory record
        let end_of_central_directory_record = EndOfCentralDirectoryRecord {
            signature: EndOfCentralDirectoryRecord::SIGNATURE,
            number_of_this_disk: if use_zip64_ending { u16::MAX } else { 0 },
            number_of_the_disk_with_the_start_of_the_central_directory: if use_zip64_ending { u16::MAX } else { 0 },
            total_number_of_entries_in_the_central_directory: if use_zip64_ending { u16::MAX } else { 1 },
            total_number_of_entries_in_the_central_directory_on_this_disk: if use_zip64_ending { u16::MAX } else { 1 },
            size_of_the_central_directory: if use_zip64_ending { u32::MAX } else { cd_size as u32 },
            offset_of_start_of_central_directory_with_respect_to_the_starting_disk_number: if use_zip64_ending { u32::MAX } else { cd_pos as u32 },
            zip_file_comment_length: 0,
        }.to_bytes();
        file.write_all(&end_of_central_directory_record)?;

        Ok(file)
    }
}

impl io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = self.deflate_writer.write(buf)?;
        self.crc_32 = crc32::run(self.crc_32, &buf[0..len]);
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.deflate_writer.flush()
    }
}


/// Read a zip file that save by `singletonzip` itself,
/// this is not a general zip file reader.
pub struct Reader {
    deflate_reader: DeflateDecoder<File>,
}
impl Reader {
    pub fn open(path: &Path) -> io::Result<Reader> {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Current(26))?;
        let file_name_length = read_u16(&mut file)?;
        file.seek(SeekFrom::Current(22 + file_name_length as i64))?;

        Ok(Self {
            deflate_reader: DeflateDecoder::new(file),
        })
    }
}

impl io::Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.deflate_reader.read(buf)
    }
}


#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    const ZERO_ZIP: &str = "test.zero.zip";
    const SMALL_ZIP: &str = "test.small.txt.zip";
    const SMALL_STR: &str = "The quick brown fox jumps over the lazy dog";

    struct PcgXshRr { // https://www.pcg-random.org
        state: u64,
    }
    impl PcgXshRr {
        const INC: u64 = 0x14057B7EF767814F;
        const MUL: u64 = 0x5851F42D4C957F2D;

        fn new(seed: u64) -> Self {
            Self { state: seed
                .wrapping_add(Self::INC)
                .wrapping_mul(Self::MUL)
                .wrapping_add(Self::INC)
            }
        }

        fn get(&mut self) -> u32 {
            let x = self.state;
            self.state = x.wrapping_mul(Self::MUL).wrapping_add(Self::INC);

            let count = x >> 59;
            let x = x ^ (x >> 18);
            let x = (x >> 27) as u32;
            x.rotate_right(count as u32)
        }
    }

    #[test]
    fn zero_write() {
        let w = Writer::create(Path::new(ZERO_ZIP)).unwrap();
        w.finish().unwrap();
    }

    #[test]
    fn small_write() {
        let mut w = Writer::create(Path::new(SMALL_ZIP)).unwrap();
        w.write_all(SMALL_STR.as_bytes()).unwrap();
        w.finish().unwrap();
    }

    /// BUG: underlying panic when using `zlib-rs` as `flate2` backend,
    /// C backends have not been tested yet (no C env in my machine).
    #[test]
    fn big_write() {
        let mut w = Writer::create(Path::new("test.big.dat.zip")).unwrap();

        let buffer_size = 32 * 1024 * 1024;
        let mut rng = PcgXshRr::new(0);
        let mut buffer = Vec::with_capacity(buffer_size);
        let mut count = 0;
        while count < u32::MAX as u64 {
            buffer.clear();
            while buffer.len() < buffer_size {
                buffer.extend_from_slice(&rng.get().to_ne_bytes());
            }

            w.write_all(&buffer).unwrap();
            count += buffer.len() as u64;
        }

        println!("count: {}", count);
        w.finish().unwrap();
    }

    #[test]
    fn zero_read() {
        zero_write();
        let mut r = Reader::open(Path::new(ZERO_ZIP)).unwrap();
        let mut buf = Vec::new();
        r.read_to_end(&mut buf).unwrap();
        assert!(buf.len() == 0);
    }

    #[test]
    fn small_read() {
        small_write();
        let mut r = Reader::open(Path::new(SMALL_ZIP)).unwrap();
        let mut s = String::new();
        r.read_to_string(&mut s).unwrap();
        assert!(s.eq(SMALL_STR));
    }
}

fn read_u16(r: &mut impl io::Read) -> io::Result<u16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}
