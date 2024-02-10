#![cfg_attr(not(test), no_std)]

use embedded_hal_async::i2c::I2c;

/// Factory assigned device address
const DEVICE_ADDRESS: u8 = 0x54;

#[derive(Debug)]
pub enum Error<I> {
    /// I2C bus error
    I2c(I),
    /// Connection error (device not found)
    Conn,
    /// Address error (invalid or out of bounds)
    Address,
    /// Port error (invalid or out of bounds)
    Port,
}

impl<I> From<I> for Error<I> {
    fn from(error: I) -> Self {
        Error::I2c(error)
    }
}

pub struct Is31Fl3218<I2C> {
    /// `embedded-hal` compatible I2C instance
    i2c: I2C,
    /// Command buffer
    cmd_buf: [u8; 23],
}

impl<I2C, E> Is31Fl3218<I2C>
where
    I2C: I2c<Error = E>,
    E: Into<Error<E>>,
{
    /// Create a new Is31Fl3218 instance
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            cmd_buf: [0; 23],
        }
    }

    async fn write_raw(&mut self, len: usize) -> Result<(), Error<E>> {
        self.i2c.write(DEVICE_ADDRESS, &self.cmd_buf[..=len]).await?;
        Ok(())
    }

    async fn write(&mut self, register: u8, values: &[u8]) -> Result<(), Error<E>> {
        let len = values.len();
        if len > 23 {
            return Err(Error::Address);
        }
        self.cmd_buf[0x0] = register;
        self.cmd_buf[0x1..=len].copy_from_slice(values);
        self.write_raw(len).await?;
        Ok(())
    }

    /// Enable the device
    /// Sets Software Shutdown Enable to Normal operation
    pub async fn enable_device(&mut self) -> Result<(), Error<E>> {
        self.write(0x0, &[0x1]).await?;
        Ok(())
    }

    /// Shutdown the device
    /// Sets Software Shutdown Enable to Software shutdown mode
    pub async fn shutdown_device(&mut self) -> Result<(), Error<E>> {
        self.write(0x0, &[0]).await?;
        Ok(())
    }

    /// Enable a channel
    /// Sets the corresponding bit in the proper LED Control Register
    pub async fn enable_channel(&mut self, led: usize) -> Result<(), Error<E>> {
        if led > 0x11 {
            return Err(Error::Address);
        }
        let register: u8 = 0x13 + led as u8 / 6;
        let bit: u8 = (led as u8 - 1) % 6;
        self.write(register, &[1 << bit]).await?;
        self.write(0x16, &[0]).await?;
        Ok(())
    }

    /// Enable all channels
    pub async fn enable_all(&mut self) -> Result<(), Error<E>> {
        self.write(0x13, &[0x3f; 3]).await?;
        self.write(0x16, &[0]).await?;
        Ok(())
    }

    /// Set one channel to a specific brightness value
    pub async fn set(&mut self, led: usize, brightness: u8) -> Result<(), Error<E>> {
        if led > 0x11 {
            return Err(Error::Address);
        }
        self.write(0x1 + led as u8, &[brightness]).await?;
        self.write(0x16, &[0]).await?;
        Ok(())
    }

    /// Set many channels to specific brightness values
    /// `start_led` starts at 0
    pub async fn set_many(&mut self, start_led: usize, values: &[u8]) -> Result<(), Error<E>> {
        let len = values.len();

        if start_led + len > 0x12 {
            return Err(Error::Address);
        }

        self.write(0x1 + start_led as u8, values).await?;
        self.write(0x16, &[0]).await?;

        Ok(())
    }

    /// Set all channels to specific brightness values and enables all channels
    pub async fn set_all(&mut self, values: &[u8; 18]) -> Result<(), Error<E>> {
        self.cmd_buf[0] = 0x1;
        self.cmd_buf[0x1..=0x12].copy_from_slice(values);
        self.cmd_buf[0x13..=0x15].copy_from_slice(&[0x3f; 3]);
        self.cmd_buf[0x16] = 0x0;
        self.write_raw(22).await?;
        Ok(())
    }

    /// Reset all registers to the default values (same as after a power cycle)
    pub async fn reset(&mut self) -> Result<(), Error<E>> {
        self.write(0x17, &[0]).await?;
        Ok(())
    }
}
