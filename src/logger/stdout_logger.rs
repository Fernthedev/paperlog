use super::LogData;

pub(crate) fn do_log(log: &LogData) {

    println!(
        "{}\n",
        log.format()
    );
}
