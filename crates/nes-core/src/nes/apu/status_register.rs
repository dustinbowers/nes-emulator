use bitflags::bitflags;

bitflags! {
    /* See: https://www.nesdev.org/wiki/APU#Status_($4015)
        7  bit  0
        ---- ----
        IF-D NT21
        |||| ||||
        |||| |||+- Pulse Channel 1 (0: disabled, 1: enabled)
        |||| ||+-- Pulse Channel 2 (0: disabled, 1: enabled)
        |||| |+--- Triangle Channel (0: disabled, 1: enabled)
        |||| +---- Noise Channel (0: disabled, 1: enabled)
        |||+------ DMC Channel (0: disabled, 1: enabled)
        ||+------- unused (technically a disconnected open-bus to CPU)
        |+-------- Frame interrupt asserted
        +--------- DMC interrupt asserted
     */
    pub struct ApuStatusRegister: u8 {
        const PULSE_CHANNEL_1 =  0b0000_0001;
        const PULSE_CHANNEL_2 =  0b0000_0010;
        const TRIANGLE_CHANNEL = 0b0000_0100;
        const NOISE_CHANNEL =    0b0000_1000;
        const DMC_CHANNEL =      0b0001_0000;
        // open bit 5            0b0010_0000;
        const FRAME_INTERRUPT =  0b0100_0000;
        // const DMC_INTERRUPT =    0b1000_0000; // Handled in DmcChannel
    }
}

impl ApuStatusRegister {
    pub fn new() -> Self {
        ApuStatusRegister::from_bits_truncate(0)
    }

    pub fn update(&mut self, data: u8) {
        *self = ApuStatusRegister::from_bits_truncate(data);
    }
}
