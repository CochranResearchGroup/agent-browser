use std::fs;

pub fn original_has_ended(pid: u32, expected: &str, observed: Result<Option<&str>, ()>) -> bool {
    match observed {
        Ok(None) => true,
        Ok(Some(actual)) if actual != expected => false,
        _ => {
            // The identity read can race exit after reading a live stat row.
            // Its missing executable link is not proof of a changed live PID.
            // Accept only positive absence or this exact original's zombie.
            let stat = match fs::read_to_string(format!("/proc/{pid}/stat")) {
                Ok(stat) => stat,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return true,
                Err(_) => return false,
            };
            let Some((_, fields)) = stat.rsplit_once(')') else {
                return false;
            };
            let fields = fields.split_whitespace().collect::<Vec<_>>();
            let (Some(&"Z"), Some(ticks)) = (fields.first(), fields.get(19)) else {
                return false;
            };
            fs::read_to_string("/proc/sys/kernel/random/boot_id")
                .is_ok_and(|boot| expected == format!("linux:{}:{ticks}", boot.trim()))
        }
    }
}
