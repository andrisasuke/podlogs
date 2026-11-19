//! Search request lifecycle and presentation state.
use crate::format::TimeRange;
use crate::models::LogSearchResponse;
use std::{collections::HashSet, rc::Rc};

#[derive(Clone, PartialEq, Debug)]
pub struct Snapshot {
    pub context: String,
    pub namespace: String,
    pub deployment: String,
    pub keyword: String,
    pub log_level: String,
    pub time_range: TimeRange,
}
#[derive(Clone, PartialEq, Debug)]
pub struct Scope {
    pub context: String,
    pub namespace: String,
    pub deployment: Option<String>,
}
impl Snapshot {
    fn in_scope(&self, scope: &Scope) -> bool {
        self.context == scope.context
            && self.namespace == scope.namespace
            && scope.deployment.as_deref() == Some(&self.deployment)
    }
}
#[derive(Clone, Debug)]
pub enum SearchStatus {
    Idle,
    Pending(Snapshot),
    Success(Snapshot, Rc<LogSearchResponse>),
    Error(Snapshot, String),
}
impl SearchStatus {
    pub fn snapshot(&self) -> Option<&Snapshot> {
        match self {
            Self::Idle => None,
            Self::Pending(s) | Self::Success(s, _) | Self::Error(s, _) => Some(s),
        }
    }
}
pub struct SearchState {
    status: SearchStatus,
    id: u64,
}
impl Default for SearchState {
    fn default() -> Self {
        Self {
            status: SearchStatus::Idle,
            id: 0,
        }
    }
}
impl SearchState {
    pub fn start(&mut self, snap: Snapshot, scope: &Scope) -> Option<u64> {
        if matches!(self.status, SearchStatus::Pending(_))
            || snap.context.is_empty()
            || snap.namespace.is_empty()
            || snap.deployment.is_empty()
            || !snap.in_scope(scope)
        {
            return None;
        }
        self.id += 1;
        self.status = SearchStatus::Pending(snap);
        Some(self.id)
    }
    pub fn finish(&mut self, id: u64, r: Result<LogSearchResponse, String>, scope: &Scope) {
        if id != self.id {
            return;
        }
        let SearchStatus::Pending(snap) = &self.status else {
            return;
        };
        if !snap.in_scope(scope) {
            return;
        }
        let snap = snap.clone();
        self.status = match r {
            Ok(data) => SearchStatus::Success(snap, Rc::new(data)),
            Err(err) => SearchStatus::Error(snap, err),
        };
    }
    pub fn reset_scope(&mut self) {
        self.id += 1;
        self.status = SearchStatus::Idle;
    }
    pub fn visible(&self, scope: &Scope) -> &SearchStatus {
        if self.status.snapshot().is_some_and(|s| !s.in_scope(scope)) {
            &SearchStatus::Idle
        } else {
            &self.status
        }
    }
    pub fn retry_snapshot(&self) -> Option<Snapshot> {
        self.status.snapshot().cloned()
    }
}
#[derive(Debug)]
pub struct SearchVm {
    pub filters_changed: bool,
    pub applied_line: Option<String>,
    pub all_failed: bool,
    pub partial: bool,
    pub total_matches: i32,
    pub unique_pods: usize,
}
pub fn view_model(status: &SearchStatus, form: &Snapshot) -> SearchVm {
    let snap = status.snapshot();
    let filters_changed = snap.is_some_and(|s| {
        s.keyword != form.keyword
            || s.log_level != form.log_level
            || s.time_range != form.time_range
    });
    let applied_line = snap.map(|s| {
        format!(
            "Applied filters: {} · {} · {}",
            if s.keyword.is_empty() {
                "All keywords".to_string()
            } else {
                format!("Keyword \"{}\"", s.keyword)
            },
            if s.log_level.is_empty() {
                "Any level"
            } else {
                &s.log_level
            },
            s.time_range.label()
        )
    });
    let data = match status {
        SearchStatus::Success(_, d) => Some(d),
        _ => None,
    };
    let all_failed = data.is_some_and(|d| d.total_containers > 0 && d.successful_containers == 0);
    let partial = data.is_some_and(|d| !d.failures.is_empty()) && !all_failed;
    let total_matches = data.map_or(0, |d| d.results.iter().map(|r| r.total_matches).sum());
    let unique_pods = data.map_or(0, |d| {
        d.results
            .iter()
            .map(|r| &r.pod_name)
            .collect::<HashSet<_>>()
            .len()
    });
    SearchVm {
        filters_changed,
        applied_line,
        all_failed,
        partial,
        total_matches,
        unique_pods,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn snap() -> Snapshot {
        Snapshot {
            context: "c".into(),
            namespace: "n".into(),
            deployment: "d".into(),
            keyword: String::new(),
            log_level: String::new(),
            time_range: TimeRange::H1,
        }
    }
    fn scope() -> Scope {
        Scope {
            context: "c".into(),
            namespace: "n".into(),
            deployment: Some("d".into()),
        }
    }
    #[test]
    fn submits_identical_params_and_empty_keyword_each_time() {
        let mut s = SearchState::default();
        let a = snap();
        let id = s.start(a.clone(), &scope()).unwrap();
        s.finish(id, Ok(Default::default()), &scope());
        assert!(s.start(a, &scope()).is_some());
    }
    #[test]
    fn refuses_duplicate_start_while_pending() {
        let mut s = SearchState::default();
        assert!(s.start(snap(), &scope()).is_some());
        assert!(s.start(snap(), &scope()).is_none());
    }
    #[test]
    fn editing_form_sets_filters_changed_but_keeps_applied_snapshot() {
        let mut s = SearchState::default();
        s.start(snap(), &scope());
        let mut form = snap();
        form.keyword = "other".into();
        let vm = view_model(s.visible(&scope()), &form);
        assert!(vm.filters_changed);
        assert!(vm.applied_line.unwrap().contains("All keywords"));
    }
    #[test]
    fn error_shown_and_retry_uses_previous_snapshot_without_auto_retry() {
        let mut s = SearchState::default();
        let id = s.start(snap(), &scope()).unwrap();
        s.finish(id, Err("failed".into()), &scope());
        assert!(matches!(s.visible(&scope()), SearchStatus::Error(_, e) if e == "failed"));
        assert_eq!(s.retry_snapshot(), Some(snap()));
    }
    #[test]
    fn ignores_late_error_after_scope_change() {
        let mut s = SearchState::default();
        let id = s.start(snap(), &scope()).unwrap();
        s.reset_scope();
        s.finish(id, Err("late".into()), &scope());
        assert!(matches!(s.visible(&scope()), SearchStatus::Idle));
    }
    #[test]
    fn timeout_error_shown_and_late_success_ignored() {
        let mut s = SearchState::default();
        let id = s.start(snap(), &scope()).unwrap();
        s.finish(id, Err("timeout".into()), &scope());
        s.finish(id, Ok(Default::default()), &scope());
        assert!(matches!(s.visible(&scope()), SearchStatus::Error(_, e) if e == "timeout"));
    }
    #[test]
    fn partial_failures_with_matches() {
        let mut response = crate::models::LogSearchResponse {
            total_containers: 2,
            successful_containers: 1,
            ..Default::default()
        };
        response.failures.push(crate::models::LogSearchFailure {
            pod_name: "p".into(),
            container_name: "c".into(),
            message: "e".into(),
        });
        response.results.push(crate::models::LogSearchResult {
            pod_name: "p".into(),
            container_name: "c".into(),
            total_matches: 3,
            entries: vec![],
        });
        let vm = view_model(&SearchStatus::Success(snap(), Rc::new(response)), &snap());
        assert!(vm.partial);
        assert_eq!((vm.total_matches, vm.unique_pods), (3, 1));
    }
    #[test]
    fn partial_without_matches_differs_from_complete_empty() {
        let mut response = crate::models::LogSearchResponse {
            total_containers: 2,
            successful_containers: 1,
            ..Default::default()
        };
        response.failures.push(crate::models::LogSearchFailure {
            pod_name: "p".into(),
            container_name: "c".into(),
            message: "e".into(),
        });
        assert!(view_model(&SearchStatus::Success(snap(), Rc::new(response)), &snap()).partial);
        assert!(!view_model(&SearchStatus::Success(snap(), Default::default()), &snap()).partial);
    }
    #[test]
    fn all_failed_is_error() {
        let response = crate::models::LogSearchResponse {
            total_containers: 1,
            ..Default::default()
        };
        let vm = view_model(&SearchStatus::Success(snap(), Rc::new(response)), &snap());
        assert!(vm.all_failed);
        assert!(!vm.partial);
    }
    #[test]
    fn zero_targets_state() {
        let vm = view_model(&SearchStatus::Success(snap(), Default::default()), &snap());
        assert!(!vm.all_failed);
    }
}
