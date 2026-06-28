use mesh_module_framework::*;

#[allow(dead_code)]
pub(crate) fn print_module_list() {

    let module_list = MMFMod::get_module_info_list().unwrap();
    println!("\n");
    println!("{}", ModuleList::get_header(ModuleOutputFormat::Terminal));

    for line in module_list.to_string_list(ModuleOutputFormat::Terminal) {
        println!("{line}");
    }
    println!("\n");
}


#[allow(dead_code)]
pub(crate) async fn print_log_list() {

    let module_list = MMFLog::wait_log(None).await.unwrap();
    println!("\n");
    println!("{}", LogList::get_header(LogOutputFormat::Terminal));

    for line in module_list.to_string_list(LogOutputFormat::Terminal) {
        println!("{line}");
    }
    println!("\n");
}


#[allow(dead_code)]
pub(crate) async fn print_error_list() {

    let module_list = MMFErr::wait_error(None).await.unwrap();
    println!("\n");
    println!("{}", ErrorList::get_header(ErrorOutputFormat::Terminal));

    for line in module_list.to_string_list(ErrorOutputFormat::Terminal) {
        println!("{line}");
    }
    println!("\n");
}