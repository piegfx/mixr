use std::fmt;

mod polymod;

use crate::{AudioFormat, binary_reader::BinaryReader};

use self::{polymod::{track::Track, track_player::TrackPlayer}};

#[derive(Debug, Clone)]
pub struct LoadError {
    pub text: String
}

impl LoadError {
    pub fn new(text: String) -> LoadError {
        LoadError { text }
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to load PCM file: {}", self.text)
    }
}

pub struct PCM {
    pub data: Vec<u8>,
    pub format: AudioFormat
}

impl PCM {
    pub fn load_wav_path(path: &str) -> Result<PCM, LoadError> {
        let read = std::fs::read(path);

        if let Ok(data) = read {
            return Self::load_wav(&data);
        }

        return Err(LoadError::new(read.err().unwrap().to_string()));
        
    }

    pub fn load_wav(data: &[u8]) -> Result<PCM, LoadError> {
        let mut reader = BinaryReader::new(data);

        if reader.read_string(4) != "RIFF" {
            return Err(LoadError::new(String::from("Given file is missing the \"RIFF\" file header. This means it is either not a wave file, or is the wrong wave type.")));
        }

        reader.read_i32();

        if reader.read_string(4) != "WAVE" {
            return Err(LoadError::new(String::from("The \"WAVE\" identifier was not found at its expected location. Currently, files with \"JUNK\" headers are not supported.")));
        }

        let mut has_data = false;
        let mut format = AudioFormat::default();
        let mut data = Vec::new();

        while !has_data {
            let text = reader.read_string(4);

            let chunk = text.as_str();
            
            match chunk {
                "fmt " => {
                    reader.read_i32();

                    let a_fmt = reader.read_i16();

                    let floating_point = if a_fmt == 1 {
                        false
                    } else if a_fmt == 3 {
                        true
                    } else {
                        return Err(LoadError::new(String::from("Unrecognized audio format.")));
                    };

                    let channels = reader.read_i16();
                    let sample_rate = reader.read_i32();

                    reader.read_i32();
                    reader.read_i16();

                    let bits_per_sample = reader.read_i16();

                    format = AudioFormat {
                        channels: channels as u8,
                        sample_rate,
                        bits_per_sample: bits_per_sample as u8,
                        floating_point
                    };
                },

                "data" => {
                    let data_size = reader.read_i32();
                    data = reader.read_bytes(data_size as usize).to_vec();

                    has_data = true;
                }

                _ => reader.position += reader.read_i32() as usize
            }
        }

        

        Ok(PCM { data, format })
    }
}

pub trait Stream {
    fn format(&self) -> AudioFormat;

    fn buffer_size(&self) -> usize;

    fn buffer(&mut self) -> Option<&[u8]>;

    fn seek(&mut self, secs: f32);

    fn seek_samples(&mut self, samples: usize);
}

pub struct StreamManager {
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn load_stream_path(&mut self, path: &str) -> Result<Box<dyn Stream + Send>, LoadError> {
        let read = std::fs::read(path);
    
        if let Ok(data) = read {
            return self.load_stream(&data);
        }
    
        return Err(LoadError::new(read.err().unwrap().to_string()));
    }

    pub fn load_stream(&mut self, data: &[u8]) -> Result<Box<dyn Stream + Send>, LoadError> {
        let mut reader = BinaryReader::new(data);
        if reader.read_string(4) == "RIFF" {
            return Ok(Box::new(PcmStream::new(data)));
        }

        reader.position = 0;
        if reader.read_string(4) == "IMPM" {
            return Ok(Box::new(TrackStream::new(Track::from_it(data).unwrap())))
        }

        reader.position = 0;
        if reader.read_string(4) == "OggS" {
            return Ok(Box::new(VorbisStream::new(data)))
        }

        reader.position = 0;

        // TODO: Actual return error.
        Err(LoadError::new(String::from("Invalid file provided.")))
    }
}

pub struct PcmStream {
    pcm: PCM,
    buffer: Vec<u8>,
    buffer_pos: usize
}

impl Stream for PcmStream {
    fn buffer(&mut self) -> Option<&[u8]> {
        let mut decoded = 0;

        for value in self.buffer.iter_mut() {
            self.buffer_pos += 1;
            if self.buffer_pos >= self.pcm.data.len() {
                break;
            }
            // It's a hack but it works!
            *value = self.pcm.data[self.buffer_pos - 1];
            decoded += 1;
        }

        if decoded == 0 {
            None
        } else {
            Some(&self.buffer[0..decoded])
        }
    }

    fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    fn format(&self) -> AudioFormat {
        self.pcm.format
    }

    fn seek(&mut self, secs: f32) {
        self.seek_samples((secs * self.pcm.format.sample_rate as f32) as usize);
    }

    fn seek_samples(&mut self, samples: usize) {
        let format = &self.pcm.format;

        self.buffer_pos = samples * format.channels as usize * (format.bits_per_sample / 8) as usize;
    }
}

impl PcmStream {
    pub fn new(data: &[u8]) -> Self {
        let pcm = PCM::load_wav(data).unwrap();

        let buffer = create_buffer(22050 * pcm.format.channels as usize * (pcm.format.bits_per_sample / 8) as usize);

        Self {
            pcm,
            buffer,
            buffer_pos: 0
        }
    }
}

pub struct TrackStream {
    player: TrackPlayer,
    buffer: Vec<u8>
}

impl Stream for TrackStream {
    fn format(&self) -> AudioFormat {
        AudioFormat { channels: 2, sample_rate: 48000, bits_per_sample: 32, floating_point: true }
    }

    fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    fn buffer(&mut self) -> Option<&[u8]> {
        for i in (0..self.buffer.len()).step_by(4) {
            let mixed = self.player.advance();
            let to_bytes = f32::to_le_bytes(mixed);
            self.buffer[i] = to_bytes[0];
            self.buffer[i + 1] = to_bytes[1];
            self.buffer[i + 2] = to_bytes[2];
            self.buffer[i + 3] = to_bytes[3];
        }

        Some(&self.buffer)
    }

    fn seek(&mut self, secs: f32) {
        todo!()
    }

    fn seek_samples(&mut self, samples: usize) {
        todo!()
    }
}

impl TrackStream {
    pub fn new(track: Track) -> Self {
        let player = TrackPlayer::new(track);
        // The sample buffer should be 22050 samples long.
        // Multiply that by 2 for the number of channels, and again by 4 as it is 32-bit.
        let buffer = create_buffer(22050 * 2 * 4);

        Self {
            player,
            buffer
        }
    }
}

pub struct VorbisStream {
    vorbis: *mut std::ffi::c_void,
    format: AudioFormat,
    buffer: Vec<u8>
}

impl Stream for VorbisStream {
    fn format(&self) -> AudioFormat {
        self.format
    }

    fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    fn buffer(&mut self) -> Option<&[u8]> {
        unsafe {
            let decoded = stb_vorbis_get_samples_float_interleaved(self.vorbis, self.format.channels as i32, self.buffer.as_mut_ptr() as *mut _, /*(self.buffer.len() / 8)*/ 100 as i32);

            if decoded <= 0 {
                None
            } else {
                Some(&self.buffer[0..decoded as usize])
            }
        }
    }

    fn seek(&mut self, secs: f32) {
        todo!()
    }

    fn seek_samples(&mut self, samples: usize) {
        todo!()
    }
}

impl VorbisStream {
    pub fn new(data: &[u8]) -> Self {
        unsafe {
            let vorbis = stb_vorbis_open_memory(data.as_ptr(), data.len() as i32, std::ptr::null_mut(), std::ptr::null_mut());

            let info = stb_vorbis_get_info(vorbis);
            let format = AudioFormat {
                channels: info.channels as u8,
                sample_rate: info.sample_rate as i32,
                bits_per_sample: 32,
                floating_point: true
            };

            let buffer = create_buffer(info.sample_rate as usize);

            stb_vorbis_seek_start(vorbis);

            Self {
                vorbis,
                format,
                buffer
            }
        }
    }
}

impl Drop for VorbisStream {
    fn drop(&mut self) {
        unsafe { std::mem::drop(Box::from_raw(self.vorbis)) }
    }
}

unsafe impl Send for VorbisStream {}

#[repr(C)]
struct StbVorbisInfo {
    pub sample_rate: std::ffi::c_uint,
    pub channels: std::ffi::c_int,

    pub setup_memory_required: std::ffi::c_uint,
    pub setup_temp_memory_required: std::ffi::c_uint,
    pub temp_memory_required: std::ffi::c_uint,

    pub nax_frame_size: std::ffi::c_int,
}

extern "C" {
    fn stb_vorbis_open_memory(data: *const std::ffi::c_uchar, len: std::ffi::c_int, error: *mut std::ffi::c_int, alloc_buffer: *const std::ffi::c_void) -> *mut std::ffi::c_void;

    fn stb_vorbis_get_info(f: *mut std::ffi::c_void) -> StbVorbisInfo;

    fn stb_vorbis_get_samples_float_interleaved(f: *mut std::ffi::c_void, channels: std::ffi::c_int, buffer: *mut std::ffi::c_float, num_floats: std::ffi::c_int) -> std::ffi::c_int;

    fn stb_vorbis_seek_start(f: *mut std::ffi::c_void);
}

fn create_buffer(size: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(size);
    for _ in 0..size {
        buffer.push(0);
    }

    buffer
}