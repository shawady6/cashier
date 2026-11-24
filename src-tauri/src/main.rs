// يمنع ظهور نافذة الكونسول في نسخة الإصدار على ويندوز.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    pos_system_lib::run()
}
