use mesh_module_framework::*;

#[allow(dead_code)]
pub(crate) fn print_module_list() {

    let module_list = MMFMod::get_module_info_list().unwrap();
    println!("{module_list}");
}


#[allow(dead_code)]
pub(crate) async fn print_log_list() {

    let log_list = MMFLog::wait_log(None).await.unwrap();
    println!("{log_list}");
}


#[allow(dead_code)]
pub(crate) async fn print_error_list() {

    let error_list = MMFErr::wait_error(None).await.unwrap();
    println!("{error_list}");
}