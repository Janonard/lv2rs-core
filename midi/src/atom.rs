/// Atom types for MIDI handling.
use crate::message::*;
use crate::prelude::*;
use crate::status_bytes::*;
use lv2rs_atom::prelude::*;
use lv2rs_urid::CachedMap;
use std::ffi::CStr;

#[repr(C)]
/// Raw representation of a "normal", non-system-exclusive message.
///
/// There are many different but similiar MIDI message types. Due to their similarities, all of them
/// are represented by this type.
///
/// `RawMidiMessage` does not have a writer extension; You simply intitialize it with a value of
/// the [`MidiMessage`](enum.MidiMessage.html) enum and that's it.
///
/// Reading is done by calling the [`interpret`](#method.interpret) method which tries to create
/// a `MidiMessage` value from the raw message.
pub struct RawMidiMessage([u8]);

impl RawMidiMessage {
    /// Try to create a `MidiMessage` from the raw message.
    ///
    /// This basically an alias for
    /// [`MidiMessage::try_from`](enum.MidiMessage.html#method.try_from) and therefore,
    /// errors are forwarded.
    pub fn interpret(&self) -> Result<MidiMessage, TryFromError> {
        MidiMessage::try_from(&self.0)
    }
}

impl<'a> AtomBody for RawMidiMessage {
    type InitializationParameter = MidiMessage;

    fn get_uri() -> &'static CStr {
        unsafe { CStr::from_bytes_with_nul_unchecked(crate::uris::EVENT_URI) }
    }

    unsafe fn initialize_body<'b, W>(
        writer: &mut W,
        message: &MidiMessage,
        _urids: &mut CachedMap,
    ) -> Result<(), ()>
    where
        W: WritingFrame<'b> + WritingFrameExt<'b, Self>,
    {
        match message {
            MidiMessage::NoteOff {
                channel,
                note,
                velocity,
            } => {
                write_channel_status(writer, NOTE_OFF_STATUS, *channel)?;
                write_data(writer, *note)?;
                write_data(writer, *velocity)?;
            }
            MidiMessage::NoteOn {
                channel,
                note,
                velocity,
            } => {
                write_channel_status(writer, NOTE_ON_STATUS, *channel)?;
                write_data(writer, *note)?;
                write_data(writer, *velocity)?;
            }
            MidiMessage::PolyKeyPressure { channel, pressure } => {
                write_channel_status(writer, POLY_KEY_PRESSURE_STATUS, *channel)?;
                write_data(writer, *pressure)?;
            }
            MidiMessage::ControlChange {
                channel,
                control_number,
                control_value,
            } => {
                write_channel_status(writer, CONTROL_CHANGE_STATUS, *channel)?;
                write_data(writer, *control_number)?;
                write_data(writer, *control_value)?;
            }
            MidiMessage::ProgramChange {
                channel,
                program_number,
            } => {
                write_channel_status(writer, PROGRAM_CHANGE_STATUS, *channel)?;
                write_data(writer, *program_number)?;
            }
            MidiMessage::ChannelPressure { channel, pressure } => {
                write_channel_status(writer, CHANNEL_PRESSURE_STATUS, *channel)?;
                write_data(writer, *pressure)?;
            }
            MidiMessage::PitchBendChange { channel, value } => {
                write_channel_status(writer, PITCH_BEND_CHANGE_STATUS, *channel)?;
                write_u14_data(writer, *value)?;
            }
            MidiMessage::TimeCodeQuarterFrame {
                message_type,
                value,
            } => {
                writer.write_sized(&TIME_CODE_QUARTER_FRAME_STATUS)?;
                let message_type: u8 = (*message_type).into();
                let value: u8 = (*value).into();
                let byte: u8 = value + (message_type << 4);
                writer.write_sized(&byte)?;
            }
            MidiMessage::SongPositionPointer { position } => {
                writer.write_sized(&SONG_POSITION_POINTER_STATUS)?;
                write_u14_data(writer, *position)?;
            }
            MidiMessage::SongSelect { song } => {
                writer.write_sized(&SONG_SELECT_STATUS)?;
                write_data(writer, *song)?;
            }
            MidiMessage::TuneRequest => {
                writer.write_sized(&TUNE_REQUEST_STATUS)?;
            }
            MidiMessage::TimingClock => {
                writer.write_sized(&TIMING_CLOCK_STATUS)?;
            }
            MidiMessage::Start => {
                writer.write_sized(&START_STATUS)?;
            }
            MidiMessage::Continue => {
                writer.write_sized(&CONTINUE_STATUS)?;
            }
            MidiMessage::Stop => {
                writer.write_sized(&STOP_STATUS)?;
            }
            MidiMessage::ActiveSensing => {
                writer.write_sized(&ACTIVE_SENSING_STATUS)?;
            }
            MidiMessage::SystemReset => {
                writer.write_sized(&SYSTEM_RESET_STATUS)?;
            }
        }
        Ok(())
    }

    fn create_ref<'b>(raw_data: &'b [u8]) -> Result<&'b Self, ()> {
        // A MIDI message may only have one, two or three bytes.
        if (raw_data.len() > 3) | (raw_data.len() == 0) {
            return Err(());
        }
        // The first byte must be a status byte.
        if (raw_data[0] & 0b1000_0000) == 0 {
            return Err(());
        }
        // The second byte must not be a status byte.
        if (raw_data.len() >= 2) & (raw_data[1] & 0b1000_0000 != 0) {
            return Err(());
        }
        // The third byte must not be a status byte.
        if (raw_data.len() == 3) & (raw_data[2] & 0b1000_0000 != 0) {
            return Err(());
        }
        // Construct and return the reference.
        let self_ptr = raw_data as *const [u8] as *const Self;
        Ok(unsafe { self_ptr.as_ref() }.unwrap())
    }
}

#[repr(C)]
/// Raw representation of a system-exclusive message.
///
/// System exclusive messages are very flexible: They start with a specific status byte and end with
/// another and anything else does not matter. However, since they have a level of flexibility that
/// other messages don't have, they have to be handled by another atom type; This one!
///
/// A `SystemExclusiveMessage` doesn't use a writing frame extension. This means that the whole
/// message has to be written in one go when initializing.
pub struct SystemExclusiveMessage([u8]);

impl SystemExclusiveMessage {
    /// Return the data bytes between the start and end status byte.
    pub fn get_data(&self) -> &[u8] {
        assert!(self.0.len() >= 2);
        let data = &self.0;
        let len = data.len();
        &data[1..len - 1]
    }
}

impl<'a> AtomBody for SystemExclusiveMessage {
    type InitializationParameter = [u8];

    fn get_uri() -> &'static CStr {
        unsafe { CStr::from_bytes_with_nul_unchecked(crate::uris::EVENT_URI) }
    }

    unsafe fn initialize_body<'b, W>(
        writer: &mut W,
        data: &[u8],
        _urids: &mut CachedMap,
    ) -> Result<(), ()>
    where
        W: WritingFrame<'b> + WritingFrameExt<'b, Self>,
    {
        writer.write_sized(&START_OF_SYSTEM_EXCLUSIVE_STATUS)?;
        writer.write_raw(data)?;
        writer.write_sized(&END_OF_SYSTEM_EXCLUSICE_STATUS)?;
        Ok(())
    }

    fn create_ref<'b>(raw_data: &'b [u8]) -> Result<&'b Self, ()> {
        // Creating the reference.
        let self_ptr = raw_data as *const [u8] as *const Self;
        let self_ref = unsafe { self_ptr.as_ref() }.unwrap();

        // Assuring a minimal length of two bytes.
        if self_ref.0.len() < 2 {
            return Err(());
        }

        // Check the first and the last byte to be the correct status bytes.
        let first_byte: u8 = *self_ref.0.first().unwrap();
        let last_byte: u8 = *self_ref.0.last().unwrap();
        if (first_byte != START_OF_SYSTEM_EXCLUSIVE_STATUS)
            | (last_byte != END_OF_SYSTEM_EXCLUSICE_STATUS)
        {
            return Err(());
        }

        // Check for interior status bytes.
        // Original MIDI allows some of them, but LV2 doesn't.
        for byte in &self_ref.0[1..self_ref.0.len() - 1] {
            if (*byte & 0b1000_0000) != 0 {
                return Err(());
            }
        }

        Ok(self_ref)
    }
}

unsafe fn write_channel_status<'a, W, A>(writer: &mut W, status: u8, channel: u4) -> Result<(), ()>
where
    W: WritingFrame<'a> + WritingFrameExt<'a, A>,
    A: AtomBody + ?Sized,
{
    let channel: u8 = channel.into();
    let status = status + channel;
    writer.write_sized(&status).map(|_| ())
}

unsafe fn write_data<'a, W, A>(writer: &mut W, data: u7) -> Result<(), ()>
where
    W: WritingFrame<'a> + WritingFrameExt<'a, A>,
    A: AtomBody + ?Sized,
{
    let data: u8 = data.into();
    writer.write_sized(&data).map(|_| ())
}

unsafe fn write_u14_data<'a, W, A>(writer: &mut W, data: u14) -> Result<(), ()>
where
    W: WritingFrame<'a> + WritingFrameExt<'a, A>,
    A: AtomBody + ?Sized,
{
    let data: u16 = data.into();
    let msb: u8 = ((data & 0b0011_1111_1000_0000) >> 7) as u8;
    let lsb: u8 = (data & 0b0000_0000_0111_1111) as u8;
    writer.write_sized(&lsb)?;
    writer.write_sized(&msb)?;
    Ok(())
}
