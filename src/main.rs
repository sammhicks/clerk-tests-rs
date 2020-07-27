use clerk::{DataPins4Lines, Pins};

struct FakeLine(&'static str);

impl clerk::DisplayHardwareLayer for FakeLine {
    fn set_level(&self, level: clerk::Level) {
        #[cfg(feature = "verbose")]
        println!("Setting {} (Fake) to {:?}", self.0, level);
        #[cfg(not(feature = "verbose"))]
        drop(level);
    }
    fn set_direction(&self, direction: clerk::Direction) {
        #[cfg(feature = "verbose")]
        println!("Setting {} (Fake) to {:?}", self.0, direction);
        #[cfg(not(feature = "verbose"))]
        drop(direction);
    }
    fn get_value(&self) -> u8 {
        #[cfg(feature = "verbose")]
        println!("Reading from {} (Fake)", self.0);
        0
    }
}

struct Line {
    #[cfg(feature = "verbose")]
    consumer: &'static str,
    handle: gpio_cdev::LineHandle,
}

impl clerk::DisplayHardwareLayer for Line {
    fn set_level(&self, level: clerk::Level) {
        #[cfg(feature = "verbose")]
        println!("Setting {} to {:?}", self.consumer, level);
        self.handle
            .set_value(match level {
                clerk::Level::Low => 0,
                clerk::Level::High => 1,
            })
            .unwrap();
    }
    fn set_direction(&self, direction: clerk::Direction) {
        #[cfg(feature = "verbose")]
        println!("Setting {} to {:?}", self.consumer, direction);
        #[cfg(not(feature = "verbose"))]
        drop(direction);
    }

    fn get_value(&self) -> u8 {
        #[cfg(feature = "verbose")]
        println!("Reading from {}", self.consumer);
        0
    }
}

struct Delay;

impl clerk::Delay for Delay {
    const ADDRESS_SETUP_TIME: u16 = 60;
    const ENABLE_PULSE_WIDTH: u16 = 450;
    const DATA_HOLD_TIME: u16 = 20;
    const COMMAND_EXECUTION_TIME: u16 = 37;

    fn delay_ns(ns: u16) {
        #[cfg(feature = "verbose")]
        println!("Sleeping for {} ns", ns);
        std::thread::sleep(std::time::Duration::from_nanos(ns as u64));
    }

    fn delay_us(us: u16) {
        #[cfg(feature = "verbose")]
        println!("Sleeping for {} µs", us);
        std::thread::sleep(std::time::Duration::from_micros(us as u64));
    }
}

fn get_line(
    chip: &mut gpio_cdev::Chip,
    offset: u32,
    consumer: &'static str,
) -> Result<Line, gpio_cdev::errors::Error> {
    let handle =
        chip.get_line(offset)?
            .request(gpio_cdev::LineRequestFlags::OUTPUT, 0, consumer)?;
    Ok(Line {
        #[cfg(feature = "verbose")]
        consumer,
        handle,
    })
}

fn main() -> Result<(), gpio_cdev::errors::Error> {
    let message = std::env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("Hello World"));

    println!("Message: {:?}", message);

    let mut chip = gpio_cdev::Chip::new("/dev/gpiochip0")?;

    let register_select = get_line(&mut chip, 17, "register_select")?;
    let read = FakeLine("read");
    let enable = get_line(&mut chip, 27, "enable")?;
    let data4 = get_line(&mut chip, 22, "data4")?;
    let data5 = get_line(&mut chip, 23, "data5")?;
    let data6 = get_line(&mut chip, 24, "data6")?;
    let data7 = get_line(&mut chip, 25, "data7")?;

    let pins = Pins {
        register_select,
        read,
        enable,
        data: DataPins4Lines {
            data4,
            data5,
            data6,
            data7,
        },
    };

    let mut lcd: clerk::Display<_, clerk::DefaultLines> =
        clerk::Display::new(pins.into_connection::<Delay>());

    #[cfg(feature = "verbose")]
    println!("init");

    lcd.init(clerk::FunctionSetBuilder::default().set_line_number(clerk::LineNumber::Two));

    #[cfg(feature = "verbose")]
    println!("set_display_control");

    lcd.set_display_control(
        clerk::DisplayControlBuilder::default()
            .set_display(clerk::DisplayState::On)
            .set_cursor(clerk::CursorState::Off)
            .set_cursor_blinking(clerk::CursorBlinking::On),
    );

    #[cfg(feature = "verbose")]
    println!("clear");

    lcd.clear();

    for c in message.chars() {
        #[cfg(feature = "verbose")]
        println!("write");
        lcd.write(c as u8);
    }

    Ok(())
}
