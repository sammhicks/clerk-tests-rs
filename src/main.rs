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
#[derive(Debug, serde::Deserialize)]
struct PinDeclarations {
    rs: u32,     // Register Select
    enable: u32, // Also known as strobe and clock
    data4: u32,
    data5: u32,
    data6: u32,
    data7: u32,
}

impl PinDeclarations {
    fn create_display(
        self,
        chip: &mut gpio_cdev::Chip,
    ) -> Result<
        clerk::Display<
            clerk::ParallelConnection<
                Line,
                FakeLine,
                Line,
                clerk::DataPins4Lines<Line, Line, Line, Line>,
                Delay,
            >,
            clerk::DefaultLines,
        >,
        gpio_cdev::errors::Error,
    > {
        let register_select = get_line(chip, self.rs, "register_select")?;
        let read = FakeLine;
        let enable = get_line(chip, self.enable, "enable")?;
        let data4 = get_line(chip, self.data4, "data4")?;
        let data5 = get_line(chip, self.data5, "data5")?;
        let data6 = get_line(chip, self.data6, "data6")?;
        let data7 = get_line(chip, self.data7, "data7")?;

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

        let lcd = clerk::Display::<_, clerk::DefaultLines>::new(pins.into_connection::<Delay>());

        lcd.init(clerk::FunctionSetBuilder::default().set_line_number(clerk::LineNumber::Two)); //screen has 4 lines, but electrically, only 2
        std::thread::sleep(std::time::Duration::from_millis(3)); //with this line commented out, screen goes blank, and cannot be written to subsequently
                                                                 //1.5 ms is marginal as 1.2ms does not work.

        lcd.set_display_control(
            clerk::DisplayControlBuilder::default() //defaults are display on cursor off blinking off ie cursor is an underscore
                .set_cursor(clerk::CursorState::On), //normally we want the cursor off
        ); //no extra delay needed here

        lcd.clear();
        std::thread::sleep(std::time::Duration::from_millis(2)); //if this line is commented out, garbage or nothing appears. 1ms is marginal

        Ok(lcd)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pins_src = std::fs::read_to_string("/boot/wiring_pins.toml")?;
    let pins: PinDeclarations = toml::from_str(&pins_src)?;

    println!("{:?}", pins);

    let mut args = std::env::args();

    let innn = args
        .next()
        .unwrap_or_else(|| String::from("useage line number offset text"));

    println!("{:?}", innn);

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
    let mut lcd = pins.create_display(&mut chip)?;

    lcd.seek_cgram(clerk::SeekFrom::Home(0)); //specify we want to write to the character generator in position 0. Must be a multiple of 8 if we want to start at the start of character
    lcd.write(31); //write top row of charcter. 1= right to righthand most dot
    lcd.write(1);
    lcd.write(1);
    lcd.write(1);
    lcd.write(1);
    lcd.write(1);
    lcd.write(0);
    lcd.write(1);

    for _count in 0..0x8 {
        lcd.write(31);
    }
    for _count in 0..0x8 {
        lcd.write(1);
    }

    let e_accute_pattern: [u8; 8] = [
        //the value 8 is used as we have an 8 by 5 character.
        0b01100, // pattern for topmost row
        0b10000, // this pattern specifies that the left-most bit is on, & the other 4 are off on the top but one row.
        0b01110, //
        0b10001, //
        0b11111, //
        0b10000, //
        0b01110, //
        0b00000, // bottom row, which is expected to be all zeros
                 // example test sequence      cargo run --release  3 0 "éèñöüäµπà~"{""["""|"""">"""
    ];

    let e_grave_pattern: [u8; 8] = [
        0b00110, //
        0b00001, //
        0b01110, //
        0b10001, //
        0b11111, //
        0b10000, //
        0b01110, //
        0b00000,
    ];

    let a_grave_pattern: [u8; 8] = [
        0b00110, //
        0b00001, //
        0b01110, //
        0b00001, //
        0b01111, //
        0b10001, //
        0b01111, //
        0b00000,
    ];

    for row in e_accute_pattern.iter() {
        lcd.write(*row);
    }
    for row in e_grave_pattern.iter() {
        lcd.write(*row);
    }
    for row in a_grave_pattern.iter() {
        lcd.write(*row);
    }

    lcd.seek(clerk::SeekFrom::Home(0)); //specify we want to write characters to be output, starting at position 0

    for count in 0..0x8 {
        //we can only specify 8 characters, so we only need to 8.
        lcd.write(count);
    }

    for c in "test".chars() {
        lcd.write(c as u8);
    }

    lcd.seek(clerk::SeekFrom::Home(line.offset() + shift));

    for unicode_character in message.chars() {
        if unicode_character < '~' {
            lcd.write(unicode_character as u8)
        } else {
            let ascii_character_bytes = match unicode_character {
                'é' => &[3],     // e accute third bespoke character defined
                'è' => &[4],     // e grave
                'à' => &[5],     // a grave
                'ä' => &[0xE1], // a umlaut            // see look up table in GDM2004D.pdf page 9/9
                'ñ' => &[0xEE], // n tilde
                'ö' => &[0xEF], // o umlaut
                'ü' => &[0xF5], // u umlaut
                'π' => &[0xE4], // pi
                'µ' => &[0xF7], // mu
                '~' => &[0xF3], // cannot display tilde using the standard character set in GDM2004D.pdf. This is the best we can do.
                '' => &[0xFF], // <Control> replaced by splodge
                _ => unidecode::unidecode_char(unicode_character).as_bytes(),
            };
            for octet in ascii_character_bytes {
                lcd.write(*octet);
            }
        }
    }

    Ok(())
}
/*



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
