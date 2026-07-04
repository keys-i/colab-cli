pub fn allowed(interactive: bool, json: bool, quiet: bool, ci: bool) -> bool {
    interactive && !json && !quiet && !ci
}
