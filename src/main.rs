use clerk::{DataPins4Lines, Pins};

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

fn main() -> Result<(), gpio_cdev::errors::Error> {
    let mut chip = gpio_cdev::Chip::new("/dev/gpiochip0")?;

    let register_select = Line(chip.get_line(2)?.request(
        gpio_cdev::LineRequestFlags::OUTPUT,
        0,
        "register_select",
    )?);
    let read = Line(
        chip.get_line(3)?
            .request(gpio_cdev::LineRequestFlags::OUTPUT, 0, "read")?,
    );
    let enable = Line(chip.get_line(4)?.request(
        gpio_cdev::LineRequestFlags::OUTPUT,
        0,
        "enable",
    )?);
    let data4 = Line(chip.get_line(16)?.request(
        gpio_cdev::LineRequestFlags::OUTPUT,
        0,
        "data4",
    )?);
    let data5 = Line(chip.get_line(19)?.request(
        gpio_cdev::LineRequestFlags::OUTPUT,
        0,
        "data5",
    )?);
    let data6 = Line(chip.get_line(26)?.request(
        gpio_cdev::LineRequestFlags::OUTPUT,
        0,
        "data6",
    )?);
    let data7 = Line(chip.get_line(20)?.request(
        gpio_cdev::LineRequestFlags::OUTPUT,
        0,
        "data7",
    )?);

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

    lcd.init(clerk::FunctionSetBuilder::default().set_line_number(clerk::LineNumber::Two));

    lcd.set_display_control(
        clerk::DisplayControlBuilder::default()
            .set_display(clerk::DisplayState::On)
            .set_cursor(clerk::CursorState::Off)
            .set_cursor_blinking(clerk::CursorBlinking::On),
    );

    lcd.write_message("Hello");

    lcd.seek(clerk::SeekFrom::Line {
        line: clerk::DefaultLines::Two,
        bytes: 5,
    });

    lcd.write_message("world!");

    Ok(())
}
