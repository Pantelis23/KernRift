use kernriftc::SurfaceProfile;

pub(crate) fn print_surface_and_count(surface: SurfaceProfile, count_label: &str, count: usize) {
    println!("surface: {}", surface.as_str());
    println!("{}: {}", count_label, count);
}

pub(crate) fn print_file_label(label: &str) {
    println!("file: {}", label);
}

pub(crate) fn print_rewrite_entry(function: &str, surface_form: &str, canonical_replacement: &str) {
    println!("function: {}", function);
    println!("surface_form: @{}", surface_form);
    println!("canonical_replacement: {}", canonical_replacement);
}

pub(crate) fn print_finding_entry(
    function: &str,
    classification: &str,
    surface_form: &str,
    canonical_replacement: &str,
    migration_safe: bool,
) {
    println!("function: {}", function);
    println!("classification: {}", classification);
    println!("surface_form: @{}", surface_form);
    println!("canonical_replacement: {}", canonical_replacement);
    println!("migration_safe: {}", migration_safe);
}

pub(crate) fn print_edit_entry(
    function: &str,
    classification: &str,
    surface_form: &str,
    canonical_replacement: &str,
    migration_safe: bool,
    rewrite_intent: &str,
) {
    print_finding_entry(
        function,
        classification,
        surface_form,
        canonical_replacement,
        migration_safe,
    );
    println!("rewrite_intent: {}", rewrite_intent);
}
