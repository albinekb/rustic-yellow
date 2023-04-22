use std::{fs::File, io::BufReader};

use rodio::{OutputStream, OutputStreamHandle, Sink};

#[derive(Debug, Copy, Clone)]
pub enum Music {
    PalletTown,
    Pokecenter,
    Gym,
    Cities1,
    Cities2,
    Celadon,
    Cinnabar,
    Vermilion,
    Lavender,
    SSAnne,
    MeetProfOak,
    MeetRival,
    MuseumGuy,
    SafariZone,
    PkmnHealed,
    Routes1,
    Routes2,
    Routes3,
    Routes4,
    IndigoPlateau,
    GymLeaderBattle,
    TrainerBattle,
    WildBattle,
    FinalBattle,
    DefeatedTrainer,
    DefeatedWildMon,
    DefeatedGymLeader,
    TitleScreen,
    Credits,
    HallOfFame,
    OaksLab,
    JigglypuffSong,
    BikeRiding,
    Surfing,
    GameCorner,
    YellowIntro,
    Dungeon1,
    Dungeon2,
    Dungeon3,
    CinnabarMansion,
    PokemonTower,
    SilphCo,
    MeetEvilTrainer,
    MeetFemaleTrainer,
    MeetMaleTrainer,
    SurfingPikachu,
    MeetJessieJames,
    YellowUnusedSong,
    GBPrinter,
}

impl Music {
    #[rustfmt::skip]
    pub fn open(&self) -> std::io::Result<File> {
        match self {
            // Bank 02
            Music::PalletTown => File::open("music/03 - Pallet Town.flac"),
            Music::Pokecenter => File::open("music/17 - Pokémon Center.flac"),
            Music::Gym => File::open("music/23 - Pokémon Gym.flac"),
            Music::Cities1 => File::open("music/16 - Pewter City.flac"),
            Music::Cities2 => File::open("music/30 - Cerulean City.flac"),
            Music::Celadon => File::open("music/41 - Celadon City.flac"),
            Music::Cinnabar => File::open("music/47 - Cinnabar Island.flac"),
            Music::Vermilion => File::open("music/35 - Vermilion City.flac"),
            Music::Lavender => File::open("music/39 - Lavender Town.flac"),
            Music::SSAnne => File::open("music/36 - S.S. Anne.flac"),
            Music::MeetProfOak => File::open("music/04 - Professor Oak.flac"),
            Music::MeetRival => File::open("music/07 - Rival.flac"),
            Music::MuseumGuy => File::open("music/21 - Hurry Along.flac"),
            Music::SafariZone => File::open("music/32 - Evolution.flac"),
            Music::PkmnHealed => File::open("music/18 - Pokémon Healed.flac"),
            Music::Routes1 => File::open("music/11 - Route 1.flac"),
            Music::Routes2 => File::open("music/31 - Route 24.flac"),
            Music::Routes3 => File::open("music/27 - Route 3.flac"),
            Music::Routes4 => File::open("music/38 - Route 11.flac"),
            Music::IndigoPlateau => File::open("music/49 - Victory Road.flac"),
            // Bank 08
            Music::GymLeaderBattle => File::open("music/25 - Battle! (Gym Leader).flac"),
            Music::TrainerBattle => File::open("music/08 - Battle! (Trainer).flac"),
            Music::WildBattle => File::open("music/13 - Battle! (Wild Pokémon).flac"),
            Music::FinalBattle => File::open("music/51 Final Battle! (Rival).flac"),
            Music::DefeatedTrainer => File::open("music/50 - Final Battle! (Rival).flac"),
            Music::DefeatedWildMon => File::open("music/14 - Victory! (Wild Pokémon).flac"),
            Music::DefeatedGymLeader => File::open("music/26 - Victory! (Gym Leader).flac"),
            // Bank 1f
            Music::TitleScreen => File::open("music/02 - Title Screen (Yellow).flac"),
            Music::Credits => File::open("music/52 - Ending.flac"),
            Music::HallOfFame => File::open("music/51 - Hall of Fame.flac"),
            Music::OaksLab => File::open("music/05 - Oak Pokémon Lab.flac"),
            Music::JigglypuffSong => File::open("music/22 - Jigglypuff's Song.flac"),
            Music::BikeRiding => File::open("music/37 - Cycling.flac"),
            Music::Surfing => File::open("music/46 - Surf.flac"),
            Music::GameCorner => File::open("music/42 - Game Corner.flac"),
            Music::YellowIntro => File::open("music/01 - Opening Movie (Yellow).flac"),
            Music::Dungeon1 => File::open("music/43 - Rocket Hideout.flac"),
            Music::Dungeon2 => File::open("music/19 - Viridian Forest.flac"),
            Music::Dungeon3 => File::open("music/29 - Mt. Moon.flac"),
            Music::CinnabarMansion => File::open("music/48 - Pokémon Mansion.flac"),
            Music::PokemonTower => File::open("music/40 - Pokémon Tower.flac"),
            Music::SilphCo => File::open("music/45 - Silph Co..flac"),
            Music::MeetEvilTrainer => File::open("music/44 - Trainers' Eyes Meet (Team Rocket).flac"),
            Music::MeetFemaleTrainer => File::open("music/28 - Trainers' Eyes Meet (Girl).flac"),
            Music::MeetMaleTrainer => File::open("music/20 - Trainers' Eyes Meet (Boy).flac"),
            // Bank 20
            Music::SurfingPikachu => File::open("music/04 - Pikachu's Beach.flac"),
            Music::MeetJessieJames => File::open("music/03 - Jessie & James.flac"),
            Music::YellowUnusedSong => File::open("music/05 - Giovanni [Hidden Track].flac"),

            // The Printer Menu track isn't part of the Soundtrack CD from the Internet Archive, so I'm using the hidden track from the CD instead
            Music::GBPrinter => File::open("music/05 - Giovanni [Hidden Track].flac"),
        }
    }
}

pub struct Sound2 {
    handle: OutputStreamHandle,
    music: Option<Sink>,
    _stream: OutputStream,
}

impl Sound2 {
    pub fn new() -> Self {
        let (stream, handle) = OutputStream::try_default().unwrap();

        Sound2 {
            _stream: stream,
            music: None,
            handle,
        }
    }

    pub fn stop_music(&mut self) {
        if let Some(sink) = self.music.take() {
            sink.stop();
        }
    }

    pub fn start_music(&mut self, id: Music) {
        self.stop_music();

        let sink = Sink::try_new(&self.handle).unwrap();
        sink.append(rodio::Decoder::new_looped(BufReader::new(id.open().unwrap())).unwrap());
        self.music = Some(sink);
    }
}