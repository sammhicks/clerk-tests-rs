use clerk::{DataPins4Lines, Pins};

struct FakeLine;

impl clerk::DisplayHardwareLayer for FakeLine {
    fn set_level(&self, _: clerk::Level) {}
    fn set_direction(&self, _: clerk::Direction) {}
    fn get_value(&self) -> u8 {
        0
    }
}

struct Line(gpio_cdev::LineHandle);

impl clerk::DisplayHardwareLayer for Line {
    fn set_level(&self, level: clerk::Level) {
        self.0
            .set_value(match level {
                clerk::Level::Low => 0,
                clerk::Level::High => 1,
            })
            .unwrap();
    }
    fn set_direction(&self, _: clerk::Direction) {}

    fn get_value(&self) -> u8 {
        0
    }
}

struct Delay;

impl clerk::Delay for Delay {
    fn delay_ns(delay: u16) {
        std::thread::sleep(std::time::Duration::from_nanos(delay as u64));
    }
}

fn get_line(
    chip: &mut gpio_cdev::Chip,
    offset: u32,
    consumer: &'static str,
) -> Result<Line, gpio_cdev::errors::Error> {
    Ok(Line(chip.get_line(offset)?.request(
        gpio_cdev::LineRequestFlags::OUTPUT,
        0,
        consumer,
    )?))
}

fn main() -> Result<(), gpio_cdev::errors::Error> {
    let message = std::env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("Hello World"));

    println!("Message: {:?}", message);

    let mut chip = gpio_cdev::Chip::new("/dev/gpiochip0")?;

    let register_select = get_line(&mut chip, 17, "register_select")?;
    let read = FakeLine;
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

    lcd.init(clerk::FunctionSetBuilder::default().set_line_number(clerk::LineNumber::One));

    std::thread::sleep(std::time::Duration::from_millis(500));

    lcd.set_display_control(
        clerk::DisplayControlBuilder::default()
            .set_display(clerk::DisplayState::On)
            .set_cursor(clerk::CursorState::Off)
            .set_cursor_blinking(clerk::CursorBlinking::On),
    );

    std::thread::sleep(std::time::Duration::from_millis(500));

    lcd.clear();

    std::thread::sleep(std::time::Duration::from_millis(500));

    for c in message.chars() {
        std::thread::sleep(std::time::Duration::from_millis(100));
        lcd.write(c as u8);
    }

    Ok(())
}
