mod build;
mod data;
mod diff;
mod edit;
mod report;

pub use build::{apply, plan, stage, Applied, Change, Choice, Opts, Stage, StagedFile, FRAME_DICT, MARGIN};
pub use data::{path_of, Data, Edit, Gap, Loc, Loose, JSON};
pub use diff::{compare, verify, Checked, Diff, Size};
pub use edit::{dict_id, edit_bytes, set_param, Req};
pub use report::{by_category, by_kind, by_reason, check_list, coverage, markdown};
