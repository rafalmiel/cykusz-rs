use crate::doomgeneric::{boolean, music_module_t, sfxinfo_t, snddevice_t, sound_module_t};
use std::os::raw::{c_int, c_void};

#[no_mangle]
pub static mut use_libsamplerate: i32 = 0;
#[no_mangle]
pub static mut libsamplerate_scale: f32 = 0.65;

unsafe impl Sync for music_module_t {}
unsafe impl Send for music_module_t {}
unsafe impl Sync for sound_module_t {}
unsafe impl Send for sound_module_t {}

static mut SOUND_DEVICES: [snddevice_t; 1] = [crate::doomgeneric::snddevice_t_SNDDEVICE_SB];

extern "C" fn music_init() -> boolean {
    if crate::doom().audio().music().init() {
        1
    } else {
        0
    }
}

extern "C" fn music_shutdown() {
    crate::doom().audio().music().shutdown()
}

extern "C" fn music_set_volume(volume: c_int) {
    crate::doom().audio().music().set_volume(volume)
}

extern "C" fn music_pause() {
    crate::doom().audio().music().pause()
}

extern "C" fn music_resume() {
    crate::doom().audio().music().resume()
}

extern "C" fn music_register_song(data: *mut c_void, len: c_int) -> *mut c_void {
    let data = unsafe { std::slice::from_raw_parts_mut(data as *mut u8, len as usize) };
    crate::doom().audio().music().register(data) as *mut c_void
}

extern "C" fn music_unregister_song(handle: *mut c_void) {
    crate::doom()
        .audio()
        .music().unregister(handle as *mut ())
}

extern "C" fn music_play_song(handle: *mut c_void, looping: boolean) {
    crate::doom()
        .audio()
        .music().play(handle as *mut (), looping == 1)
}

extern "C" fn music_stop_song() {
    crate::doom().audio().music().stop_song()
}

extern "C" fn music_is_playing() -> boolean {
    if crate::doom().audio().music().is_playing() {
        1
    } else {
        0
    }
}

extern "C" fn music_poll() {
    crate::doom().audio().poll()
}

#[no_mangle]
static DG_music_module: music_module_t = music_module_t {
    sound_devices: unsafe { SOUND_DEVICES }.as_ptr() as *mut snddevice_t,
    num_sound_devices: unsafe { SOUND_DEVICES }.len() as c_int,
    Init: Some(music_init),
    Shutdown: Some(music_shutdown),
    SetMusicVolume: Some(music_set_volume),
    PauseMusic: Some(music_pause),
    ResumeMusic: Some(music_resume),
    RegisterSong: Some(music_register_song),
    UnRegisterSong: Some(music_unregister_song),
    PlaySong: Some(music_play_song),
    StopSong: Some(music_stop_song),
    MusicIsPlaying: Some(music_is_playing),
    Poll: Some(music_poll),
};

extern "C" fn sound_init(use_sfx_prefix: boolean) -> boolean {
    if crate::doom().audio().sound().init(use_sfx_prefix == 1) {
        1
    } else {
        0
    }
}

extern "C" fn sound_shutdown() {
    crate::doom().audio().sound().shutdown()
}

extern "C" fn sound_get_sfx_lumpnum(sfxinfo: *mut sfxinfo_t) -> c_int {
    crate::doom()
        .audio()
        .sound().sfx_lumpnum(unsafe { sfxinfo.as_mut_unchecked() })
}

extern "C" fn sound_update() {
    crate::doom().audio().sound().update()
}
extern "C" fn sound_update_params(channel: c_int, vol: c_int, sep: c_int) {
    crate::doom().audio().sound().update_params(channel, vol, sep);
}

extern "C" fn sound_start(
    sfxinfo: *mut sfxinfo_t,
    channel: c_int,
    vol: c_int,
    sep: c_int,
) -> c_int {
    crate::doom()
        .audio()
        .sound().start(unsafe { sfxinfo.as_mut_unchecked() }, channel, vol, sep)
}

extern "C" fn sound_stop(channel: c_int) {
    crate::doom().audio().sound().stop(channel);
}

extern "C" fn sound_is_playing(channel: c_int) -> boolean {
    if crate::doom().audio().sound().is_playing(channel) {
        1
    } else {
        0
    }
}

extern "C" fn sound_cache(sounds: *mut sfxinfo_t, num: c_int) {
    let sounds = unsafe { std::slice::from_raw_parts_mut(sounds, num as usize) };

    crate::doom().audio().sound().cache(sounds);
}

#[no_mangle]
static DG_sound_module: sound_module_t = sound_module_t {
    sound_devices: unsafe { SOUND_DEVICES }.as_ptr() as *mut snddevice_t,
    num_sound_devices: unsafe { SOUND_DEVICES }.len() as c_int,
    Init: Some(sound_init),
    Shutdown: Some(sound_shutdown),
    GetSfxLumpNum: Some(sound_get_sfx_lumpnum),
    Update: Some(sound_update),
    UpdateSoundParams: Some(sound_update_params),
    StartSound: Some(sound_start),
    StopSound: Some(sound_stop),
    SoundIsPlaying: Some(sound_is_playing),
    CacheSounds: Some(sound_cache),
};
