use clerk::{DataPins4Lines, Pins};

enum LCDLineNumbers {
    Line1,
    Line2,
    Line3,
    Line4,
}

impl LCDLineNumbers {
    const NUM_CHARACTERS_PER_LINE: u8 = 20;
    const ROW_OFFSET: u8 = 0x40;

    fn offset(self) -> u8 {
        match self {
            LCDLineNumbers::Line1 => 0,
            LCDLineNumbers::Line2 => Self::ROW_OFFSET,
            LCDLineNumbers::Line3 => Self::NUM_CHARACTERS_PER_LINE,
            LCDLineNumbers::Line4 => Self::ROW_OFFSET + Self::NUM_CHARACTERS_PER_LINE,
        }
    }
}

struct FakeLine;

impl clerk::DisplayHardwareLayer for FakeLine {
    fn set_level(&self, _level: clerk::Level) {}
    fn set_direction(&self, _direction: clerk::Direction) {}
    fn get_value(&self) -> u8 {
        0
    }
}

struct Line {
    handle: gpio_cdev::LineHandle,
}

impl clerk::DisplayHardwareLayer for Line {
    fn set_level(&self, level: clerk::Level) {
        self.handle
            .set_value(match level {
                clerk::Level::Low => 0,
                clerk::Level::High => 1,
            })
            .unwrap();
    }
    fn set_direction(&self, _direction: clerk::Direction) {}

    fn get_value(&self) -> u8 {
        0
    }
}

struct Delay;

impl clerk::Delay for Delay {
    const ADDRESS_SETUP_TIME: u16 = 60;
    const ENABLE_PULSE_WIDTH: u16 = 300; //300ns in the spec sheet 450;
    const DATA_HOLD_TIME: u16 = 10; //10ns in the spec sheet  20;
    const COMMAND_EXECUTION_TIME: u16 = 37;

    fn delay_ns(ns: u16) {
        std::thread::sleep(std::time::Duration::from_nanos(ns as u64));
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
    Ok(Line { handle })
}

fn main() -> Result<(), gpio_cdev::errors::Error> {
    // uses BCM GPIO pin numbers
    let screen_rs = 17; //     GPIO 0  Physical pin 11 RS ie reset goes to display pin 4
    let screen_enable = 27; // GPIO 2  physical pin 13 E ie Strobe goes to display pin 6 (also known as Enable)
    let screen_data4 = 22; //  GPIO 3  physical pin 15	           goes to display pin 11
    let screen_data5 = 23; //  GPIO 4  physical pin 16 	           goes to display pin 12
    let screen_data6 = 24; //  GPIO 5  physical pin 18	           goes to display pin 13
    let screen_data7 = 25; //  GPIO 6  physical pin 22	           goes to display pin 14
                           //          physical pin 25 & 6 gnd     goes to display 1
                           //                           brightness goes to display 3
                           //          physical pin 2     +5       goes to display 2
                           //                             gnd      goes to display 16 (lighing gnd))
                           //          physical pin 4   dispay +5  goes to dispaly 15 (Anode lighting diode power)

    println!(
        "RS pin {} enable pin {} D4...7 {}, {}, {}, {}",
        screen_rs, screen_enable, screen_data4, screen_data5, screen_data6, screen_data7
    );

    let mut args = std::env::args().skip(1);
    let line_as_integer = args.next().map(|shift| shift.parse().unwrap()).unwrap_or(0);

    let line = match line_as_integer {
        1 => LCDLineNumbers::Line1,
        2 => LCDLineNumbers::Line2,
        3 => LCDLineNumbers::Line3,
        _ => LCDLineNumbers::Line4,
    };

    let shift = args.next().map(|shift| shift.parse().unwrap()).unwrap_or(0);
    let message = args
        .next()
        .unwrap_or_else(|| String::from("useage line number offset text"));

    println!("Message: {:?}", message);

    let mut chip = gpio_cdev::Chip::new("/dev/gpiochip0")?; //no delay needed here
    let register_select = get_line(&mut chip, screen_rs, "register_select")?;
    let read = FakeLine;
    let enable = get_line(&mut chip, screen_enable, "enable")?;
    let data4 = get_line(&mut chip, screen_data4, "data4")?;
    let data5 = get_line(&mut chip, screen_data5, "data5")?;
    let data6 = get_line(&mut chip, screen_data6, "data6")?;
    let data7 = get_line(&mut chip, screen_data7, "data7")?;

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

    let mut lcd = clerk::Display::<_, clerk::DefaultLines>::new(pins.into_connection::<Delay>()); //no extra delay needed here

    lcd.init(clerk::FunctionSetBuilder::default().set_line_number(clerk::LineNumber::Two)); //screen has 4 lines, but electrically, only 2
    std::thread::sleep(std::time::Duration::from_millis(3)); //with this line commented out, screen goes blank, and cannot be written to subsequently
                                                             //1.5 ms is marginal as 1.2ms does not work.

    lcd.set_display_control(
        clerk::DisplayControlBuilder::default() //defaults are display on cursor off blinking off ie cursor is an underscore
            .set_cursor(clerk::CursorState::On), //normally we want the cursor off
    ); //no extra delay needed here

    lcd.clear();
    std::thread::sleep(std::time::Duration::from_millis(2)); //if this line is commented out, garbage or nothing appears. 1ms is marginal

    for c in "test2".chars() {
        lcd.write(c as u8);
    }

    lcd.seek(clerk::SeekFrom::Home(line.offset() + shift));

    for c in message.chars() {
        let cc = match c {
            ' '..='}' => c as u8,
            'ä' => 0xE1,
            'ñ' => 0xEE,
            'ö' => 0xEF,
            'ü' => 0xF5,
            'π' => 0xE4,
            'µ' => 0xF7,
            _ => 0xFF, // solid square used when the decode fails
        };
        lcd.write(cc as u8);
    }

    Ok(())
}
/*
const unsigned char e_accute_pattern[8] =
    {
        0b01100,
        0b10000,
        0b01110,
        0b10001,
        0b11111,
        0b10000,
        0b01110,
        0b00000};

const unsigned char e_grave_pattern[8] =
    {
        0b00110,
        0b00001,
        0b01110,
        0b10001,
        0b11111,
        0b10000,
        0b01110,
        0b00000};

const unsigned char buffer_1_pattern[8] =
    {
        0b10000,
        0b10000,
        0b10000,
        0b10000,
        0b10000,
        0b10000,
        0b10000,
        0b11111};
const unsigned char buffer_2_pattern[8] =
    {
        0b01000,
        0b01000,
        0b01000,
        0b01000,
        0b01000,
        0b01000,
        0b01000,
        0b11111};
const unsigned char buffer_3_pattern[8] =
    {
        0b00100,
        0b00100,
        0b00100,
        0b00100,
        0b00100,
        0b00100,
        0b00100,
        0b11111};
const unsigned char buffer_4_pattern[8] =
    {
        0b00010,
        0b00010,
        0b00010,
        0b00010,
        0b00010,
        0b00010,
        0b00010,
        0b11111};
const unsigned char buffer_5_pattern[8] =
    {
        0b00001,
        0b00001,
        0b00001,
        0b00001,
        0b00001,
        0b00001,
        0b00001,
        0b11111};
*/
