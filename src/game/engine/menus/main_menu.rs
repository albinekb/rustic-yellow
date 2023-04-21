use crate::{
    cpu::Cpu,
    game::{
        constants, home, macros,
        ram::{sram, wram},
    },
    saves, KeypadKey,
};

pub fn main_menu(cpu: &mut Cpu) {
    // FIXME: Implement our own audio system that isn't dependent of the CPU cycling
    cpu.call(0x2233); // StopAllMusic

    init_options(cpu);

    cpu.write_byte(wram::W_OPTIONS_INITIALIZED, 0);

    let has_saves = match saves::list_save_files() {
        Ok(files) => !files.is_empty(),
        Err(e) => {
            eprintln!("Error listing save files: {}", e);
            false
        }
    };

    cpu.write_byte(wram::W_SAVE_FILE_STATUS, if has_saves { 2 } else { 1 });

    cpu.write_byte(
        wram::W_LINK_STATE,
        constants::serial_constants::LINK_STATE_NONE,
    );

    cpu.write_byte(wram::W_PARTY_AND_BILLS_PC_SAVED_MENU_ITEM, 0);
    cpu.write_byte(wram::W_PARTY_AND_BILLS_PC_SAVED_MENU_ITEM + 1, 0);
    cpu.write_byte(wram::W_PARTY_AND_BILLS_PC_SAVED_MENU_ITEM + 2, 0);
    cpu.write_byte(wram::W_PARTY_AND_BILLS_PC_SAVED_MENU_ITEM + 3, 0);

    cpu.write_byte(wram::W_DEFAULT_MAP, 0);

    // Toggle link feature bit off
    {
        let v = cpu.read_byte(wram::W_D72E);
        cpu.write_byte(wram::W_D72E, v & !(1 << 6));
    }

    cpu.stack_push(0x0001);
    home::palettes::run_default_palette_command(cpu);

    cpu.call(0x36a3); // call LoadTextBoxTilePatterns
    cpu.call(0x3683); // call LoadFontTilePatterns

    let layer = cpu.gpu_push_layer();

    if has_saves {
        home::text::text_box_border(cpu.gpu_mut_layer(layer), 0, 0, 13, 6);
        home::text::place_string(cpu.gpu_mut_layer(layer), 2, 2, "CONTINUE");
        home::text::place_string(cpu.gpu_mut_layer(layer), 2, 4, "NEW GAME");
        home::text::place_string(cpu.gpu_mut_layer(layer), 2, 6, "OPTION");
    } else {
        home::text::text_box_border(cpu.gpu_mut_layer(layer), 0, 0, 13, 4);
        home::text::place_string(cpu.gpu_mut_layer(layer), 2, 2, "NEW GAME");
        home::text::place_string(cpu.gpu_mut_layer(layer), 2, 4, "OPTION");
    }

    let mut selected = 0;
    let max_menu_item = if has_saves { 2 } else { 1 };

    loop {
        cpu.gpu_mut_layer(layer)
            .set_background(1, selected * 2 + 2, home::text::SELECTED_ITEM);

        cpu.gpu_update_screen();
        let key = cpu.keypad_wait();

        match key {
            KeypadKey::B => {
                cpu.gpu_pop_layer(layer);
                return cpu.jump(0x4171); // jump DisplayTitleScreen
            }

            KeypadKey::Up if selected > 0 => {
                cpu.gpu_mut_layer(layer)
                    .set_background(1, selected * 2 + 2, home::text::EMPTY);
                selected -= 1;
                continue;
            }

            KeypadKey::Down if selected < max_menu_item => {
                cpu.gpu_mut_layer(layer)
                    .set_background(1, selected * 2 + 2, home::text::EMPTY);
                selected += 1;
                continue;
            }

            KeypadKey::A => {}
            _ => {
                continue;
            }
        }

        // If there's no save file, increment the current menu item so that the numbers
        // are the same whether or not there's a save file.
        let selected = if has_saves { selected } else { selected + 1 };

        match selected {
            0 => {
                if main_menu_select_save(cpu) {
                    cpu.gpu_pop_layer(layer);
                    return cpu.jump(0x5c83); // MainMenu.pressedA
                }
            }
            1 => {
                cpu.gpu_pop_layer(layer);
                return cpu.jump(0x5cd2); // StartNewGame
            }
            2 => {
                cpu.gpu_pop_layer(layer);
                cpu.call(0x5df2); // DisplayOptionMenu
                cpu.write_byte(wram::W_OPTIONS_INITIALIZED, 1);
                return cpu.jump(0x4171); // jump DisplayTitleScreen
            }
            _ => unreachable!("Invalid menu item: {}", selected),
        }
    }
}

fn main_menu_select_save(cpu: &mut Cpu) -> bool {
    let list = match saves::list_save_files() {
        Ok(ref files) if files.is_empty() => {
            return false;
        }
        Ok(files) => files,
        Err(error) => {
            eprintln!("Error listing save files: {}", error);
            return false;
        }
    };

    let first_page = &list[0..8.min(list.len())];
    let height = first_page.len() * 2;

    let layer = cpu.gpu_push_layer();
    home::text::text_box_border(cpu.gpu_mut_layer(layer), 0, 0, 18, height);

    for (i, save) in first_page.iter().enumerate() {
        home::text::place_string(cpu.gpu_mut_layer(layer), 2, i * 2 + 2, &save.name);
    }

    let mut selected = 0;

    loop {
        cpu.gpu_mut_layer(layer)
            .set_background(1, selected * 2 + 2, home::text::SELECTED_ITEM);

        cpu.gpu_update_screen();
        let key = cpu.keypad_wait();

        match key {
            KeypadKey::B => {
                cpu.gpu_pop_layer(layer);
                return false;
            }

            KeypadKey::Up if selected > 0 => {
                cpu.gpu_mut_layer(layer)
                    .set_background(1, selected * 2 + 2, home::text::EMPTY);
                selected -= 1;
                continue;
            }

            KeypadKey::Down if selected < first_page.len() - 1 => {
                cpu.gpu_mut_layer(layer)
                    .set_background(1, selected * 2 + 2, home::text::EMPTY);
                selected += 1;
                continue;
            }

            KeypadKey::A => {}
            _ => {
                continue;
            }
        }

        let save = &list[selected];

        cpu.replace_ram(std::fs::read(&save.path).unwrap());

        macros::predef::predef_call!(cpu, LoadSAV);

        if display_continue_game_info(cpu) {
            cpu.gpu_pop_layer(layer);
            return true;
        }
    }
}

fn display_continue_game_info(cpu: &mut Cpu) -> bool {
    let name = check_for_player_name_in_sram(cpu);
    let badges = cpu.read_byte(wram::W_OBTAINED_BADGES).count_ones();
    let num_owned = read_num_owned_mons(cpu);
    let hours = cpu.read_byte(wram::W_PLAY_TIME_HOURS);
    let minutes = cpu.read_byte(wram::W_PLAY_TIME_MINUTES);

    let layer = cpu.gpu_push_layer();

    home::text::text_box_border(cpu.gpu_mut_layer(layer), 4, 7, 14, 8);

    home::text::place_string(cpu.gpu_mut_layer(layer), 5, 9, "PLAYER");
    home::text::place_string(cpu.gpu_mut_layer(layer), 12, 9, &name);

    home::text::place_string(cpu.gpu_mut_layer(layer), 5, 11, "BADGES");
    home::text::place_string(cpu.gpu_mut_layer(layer), 17, 11, &format!("{:2}", badges));

    home::text::place_string(cpu.gpu_mut_layer(layer), 5, 13, "POKéDEX");
    home::text::place_string(
        cpu.gpu_mut_layer(layer),
        16,
        13,
        &format!("{:3}", num_owned),
    );

    home::text::place_string(cpu.gpu_mut_layer(layer), 5, 15, "TIME");
    home::text::place_string(
        cpu.gpu_mut_layer(layer),
        13,
        15,
        &format!("{:3}:{:02}", hours, minutes),
    );

    cpu.gpu_update_screen();

    let result = loop {
        match cpu.keypad_wait() {
            KeypadKey::A => {
                break true;
            }
            KeypadKey::B => {
                break false;
            }
            _ => {}
        }
    };

    cpu.gpu_pop_layer(layer);
    result
}

/// Check if the player name data in SRAM has a string terminator character
/// (indicating that a name may have been saved there) and return whether it does
pub fn check_for_player_name_in_sram(cpu: &mut Cpu) -> String {
    cpu.write_byte(
        constants::hardware_constants::MBC1_SRAM_ENABLE,
        constants::hardware_constants::SRAM_ENABLE,
    );
    cpu.write_byte(constants::hardware_constants::MBC1_SRAM_BANKING_MODE, 1);
    cpu.write_byte(constants::hardware_constants::MBC1_SRAM_BANK, 1);

    let mut result = String::with_capacity(constants::text_constants::NAME_LENGTH as usize);

    for i in 0..=constants::text_constants::NAME_LENGTH {
        let ch = cpu.read_byte(sram::S_PLAYER_NAME + (i as u16));

        match ch {
            0x50 => {
                break;
            }
            0x80..=0x99 => {
                result.push((('A' as u8) + (ch - 0x80)) as char);
            }
            0xa0..=0xb9 => {
                result.push((('a' as u8) + (ch - 0xa0)) as char);
            }
            0xf6..=0xff => {
                result.push((('0' as u8) + (ch - 0xf6)) as char);
            }
            _ => panic!("Invalid character in player name: {:02x}", ch),
        }
    }

    cpu.write_byte(
        constants::hardware_constants::MBC1_SRAM_ENABLE,
        constants::hardware_constants::SRAM_DISABLE,
    );
    cpu.write_byte(constants::hardware_constants::MBC1_SRAM_BANKING_MODE, 0);

    result
}

fn read_num_owned_mons(cpu: &mut Cpu) -> u32 {
    let mut num_owned = 0;

    for addr in wram::W_POKEDEX_OWNED..wram::W_POKEDEX_OWNED_END {
        let byte = cpu.read_byte(addr);
        num_owned += byte.count_ones();
    }

    num_owned
}

pub fn init_options(cpu: &mut Cpu) {
    cpu.write_byte(
        wram::W_LETTER_PRINTING_DELAY_FLAGS,
        constants::misc_constants::TEXT_DELAY_FAST,
    );
    cpu.write_byte(
        wram::W_OPTIONS,
        constants::misc_constants::TEXT_DELAY_MEDIUM,
    );
    cpu.write_byte(wram::W_PRINTER_SETTINGS, 64); // audio?
}
