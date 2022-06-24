#![cfg_attr(not(test), no_std)]

use embedded_hal::blocking::i2c::{Read, Write, WriteRead};

/// Factory assigned device address
const DEVICE_ADDRESS: u8 = 0x54;

#[derive(Debug)]
pub enum Error<I> {
    /// I2C bus error
    I2C(I),
    /// Connection error (device not found)
    Conn,
    /// Address error (invalid or out of bounds)
    Address,
    /// Port error (invalid or out of bounds)
    Port,
}

pub struct Is31Fl3218<I2C> {
    /// `embedded-hal` compatible I2C instance
    i2c: I2C,
    /// Command buffer
    cmd_buf: [u8; 23],
}

impl<'a, I2C, S> Is31Fl3218<I2C>
where
    I2C: Write<u8, Error = S> + Read<u8, Error = S> + WriteRead<u8, Error = S>,
{
    /// Create a new Is31Fl3218 instance. Will enable the device immediately
    pub fn new(i2c: I2C) -> Result<Self, Error<S>> {
        let mut is31fl3218 = Self {
            i2c,
            cmd_buf: [0; 23],
        };

        is31fl3218.enable_device()?;

        Ok(is31fl3218)
    }

    fn write_raw(&mut self, len: usize) -> Result<(), Error<S>> {
        self.i2c
            .write(DEVICE_ADDRESS, &self.cmd_buf[..=len])
            .map_err(Error::I2C)?;
        Ok(())
    }

    fn write(&mut self, register: u8, values: &[u8]) -> Result<(), Error<S>> {
        let len = values.len();
        if len > 23 {
            return Err(Error::Address);
        }
        self.cmd_buf[0x0] = register;
        self.cmd_buf[0x1..=len].copy_from_slice(values);
        self.write_raw(len)?;
        Ok(())
    }

    /// Enable the device
    /// Sets Software Shutdown Enable to Normal operation
    pub fn enable_device(&mut self) -> Result<(), Error<S>> {
        self.write(0x0, &[0x1])?;
        Ok(())
    }

    /// Shutdown the device
    /// Sets Software Shutdown Enable to Software shutdown mode
    pub fn shutdown_device(&mut self) -> Result<(), Error<S>> {
        self.write(0x0, &[0])?;
        Ok(())
    }

    /// Enable a channel
    /// Sets the corresponding bit in the proper LED Control Register
    pub fn enable_channel(&mut self, led: usize) -> Result<(), Error<S>> {
        if led > 0x11 {
            return Err(Error::Address);
        }
        let register: u8 = 0x13 + led as u8 / 6;
        let bit: u8 = (led as u8 - 1) % 6;
        self.write(register, &[1 << bit])?;
        self.write(0x16, &[0])?;
        Ok(())
    }

    /// Enable all channels
    pub fn enable_all(&mut self) -> Result<(), Error<S>> {
        self.write(0x13, &[0x3f; 3])?;
        self.write(0x16, &[0])?;
        Ok(())
    }

    /// Set one channel to a specific brightness value
    pub fn set(&mut self, led: usize, brightness: u8) -> Result<(), Error<S>> {
        if led > 0x11 {
            return Err(Error::Address);
        }
        self.write(0x1 + led as u8, &[brightness])?;
        self.write(0x16, &[0])?;
        Ok(())
    }

    /// Set many channels to specific brightness values
    /// `start_led` starts at 0
    pub fn set_many(&mut self, start_led: usize, values: &[u8]) -> Result<(), Error<S>> {
        let len = values.len();

        if start_led + len > 0x12 {
            return Err(Error::Address);
        }

        self.write(0x1 + start_led as u8, values)?;
        self.write(0x16, &[0])?;

        Ok(())
    }

    /// Set all channels to specific brightness values and enables all channels
    pub fn set_all(&mut self, values: &[u8; 18]) -> Result<(), Error<S>> {
        self.cmd_buf[0] = 0x1;
        self.cmd_buf[0x1..=0x12].copy_from_slice(values);
        self.cmd_buf[0x13..=0x15].copy_from_slice(&[0x3f; 3]);
        self.cmd_buf[0x16] = 0x0;
        self.write_raw(22)?;
        Ok(())
    }

    /// Reset all registers to the default values (same as after a power cycle)
    pub fn reset(&mut self) -> Result<(), Error<S>> {
        self.write(0x17, &[0])?;
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    use embedded_hal_mock::i2c::{Mock as I2cMock, Transaction as I2cTransaction};

    #[test]
    fn init() {
        let mut i2c = I2cMock::new([]);
        i2c.expect(&[
            // Device enable
            I2cTransaction::write(DEVICE_ADDRESS, vec![0, 1]),
            // LED enable (0x15 - 21, bit 2 - 4)
            I2cTransaction::write(DEVICE_ADDRESS, vec![21, 4]),
            // Flush for LED enable
            I2cTransaction::write(DEVICE_ADDRESS, vec![22, 0]),
            // Write all
            I2cTransaction::write(
                DEVICE_ADDRESS,
                vec![
                    1, // Start at register 0x1
                    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                    255, 255, // 18 x 255 brightness values
                    63, 63, 63, // Set all three control registers to 111111
                    0,  // And flush the update register
                ],
            ),
        ]);
        let mut led_driver = Is31Fl3218::new(i2c).unwrap();
        led_driver.enable_channel(15).unwrap();
        led_driver.set_all(&[255; 18]).unwrap();
    }
}
