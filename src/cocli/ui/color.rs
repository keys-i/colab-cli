pub fn enabled(no_color: bool, json: bool, quiet: bool) -> bool {
    !no_color && !json && !quiet
}
