mod common;

#[test]
fn test_common() {
   let project_dir = common::get_project_dir(); 
   dbg!(project_dir);
}
