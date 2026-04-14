use std::{
    collections::HashMap,
    hash::{BuildHasher, RandomState},
    io,
    sync::{
        LazyLock,
        mpsc::{self, RecvTimeoutError, SyncSender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use kitest::formatter::{
    FmtTestOutcome, FmtTestStart, GroupedTestFormatter, TestFormatter,
    common::{
        color::ColorSetting,
        label::{FromGroupCtx, GroupLabel},
    },
    pretty::PrettyFormatter,
};

use crate::harness::{
    Extra,
    group::{GroupCtx, GroupKey},
};

static RANDOM_STATE: LazyLock<RandomState> = LazyLock::new(|| RandomState::new());

type InnerFormatter<'t> = PrettyFormatter<'t, io::Stdout, GroupLabel<FromGroupCtx>, Extra>;

#[derive(Debug)]
pub struct ProgressFormatter<'t> {
    formatter: InnerFormatter<'t>,
    multi_progress: MultiProgress,
    progress_bars: HashMap<&'t str, ProgressBar>,
    ticker: Option<JoinHandle<()>>,
    ticker_tx: SyncSender<TickerEvent>,
}

impl<'t> ProgressFormatter<'t> {
    pub fn with_color_setting(self, color_setting: impl Into<ColorSetting>) -> Self {
        Self {
            formatter: self.formatter.with_color_setting(color_setting),
            ..self
        }
    }
}

#[derive(Debug)]
enum TickerEvent {
    Add { id: u64, pb: ProgressBar },
    Remove { id: u64 },
}

impl<'t> Default for ProgressFormatter<'t> {
    fn default() -> Self {
        let (ticker_tx, ticker_rx) = mpsc::sync_channel(32);
        Self {
            formatter: PrettyFormatter::default().with_group_label_from_ctx(),
            multi_progress: MultiProgress::with_draw_target(ProgressDrawTarget::stdout()),
            progress_bars: Default::default(),
            ticker: Some(thread::spawn(move || {
                let mut pbs = HashMap::with_capacity(16);
                loop {
                    match ticker_rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(TickerEvent::Add { id, pb }) => drop(pbs.insert(id, pb)),
                        Ok(TickerEvent::Remove { id }) => drop(pbs.remove(&id)),
                        Err(RecvTimeoutError::Disconnected) => break,
                        Err(RecvTimeoutError::Timeout) => (),
                    };

                    pbs.values().for_each(|pb| pb.tick());
                }
            })),
            ticker_tx,
        }
    }
}

pub struct TestStart<'t> {
    test_name: &'t str,
    for_formatter: <InnerFormatter<'t> as TestFormatter<'t, Extra>>::TestStart,
}

impl<'t> From<FmtTestStart<'t, Extra>> for TestStart<'t> {
    fn from(value: FmtTestStart<'t, Extra>) -> Self {
        TestStart {
            test_name: value.meta.name.as_ref(),
            for_formatter: value.into(),
        }
    }
}

pub struct TestOutcome<'t> {
    test_name: &'t str,
    for_formatter: <InnerFormatter<'t> as TestFormatter<'t, Extra>>::TestOutcome,
}

impl<'t> From<FmtTestOutcome<'t, '_, Extra>> for TestOutcome<'t> {
    fn from(value: FmtTestOutcome<'t, '_, Extra>) -> Self {
        TestOutcome {
            test_name: value.meta.name.as_ref(),
            for_formatter: value.into(),
        }
    }
}

impl<'t> TestFormatter<'t, Extra> for ProgressFormatter<'t> {
    type Error = io::Error;
    type TestStart = TestStart<'t>;
    type TestOutcome = TestOutcome<'t>;

    type RunInit = <InnerFormatter<'t> as TestFormatter<'t, Extra>>::RunInit;
    type RunStart = <InnerFormatter<'t> as TestFormatter<'t, Extra>>::RunStart;
    type TestIgnored = <InnerFormatter<'t> as TestFormatter<'t, Extra>>::TestIgnored;
    type RunOutcomes = <InnerFormatter<'t> as TestFormatter<'t, Extra>>::RunOutcomes;

    fn fmt_run_init(&mut self, data: Self::RunInit) -> Result<(), Self::Error> {
        self.formatter.fmt_run_init(data)
    }

    fn fmt_run_start(&mut self, data: Self::RunStart) -> Result<(), Self::Error> {
        self.formatter.fmt_run_start(data)
    }

    fn fmt_test_ignored(&mut self, data: Self::TestIgnored) -> Result<(), Self::Error> {
        self.formatter.fmt_test_ignored(data)
    }

    fn fmt_test_start(&mut self, data: Self::TestStart) -> Result<(), Self::Error> {
        let id = RANDOM_STATE.hash_one(data.test_name);
        let pb = ProgressBar::new_spinner()
            .with_message(format!("test {}", data.test_name))
            .with_style(
                ProgressStyle::default_spinner()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                    .template("{msg} {spinner}")
                    .expect("valid template"),
            );
        self.multi_progress.add(pb.clone());
        self.progress_bars.insert(data.test_name, pb.clone());
        self.ticker_tx
            .send(TickerEvent::Add { id, pb })
            .expect("ticker disconnected");

        self.multi_progress
            .suspend(|| self.formatter.fmt_test_start(data.for_formatter))
    }

    fn fmt_test_outcome(&mut self, data: Self::TestOutcome) -> Result<(), Self::Error> {
        let id = RANDOM_STATE.hash_one(data.test_name);
        if let Some(pb) = self.progress_bars.remove(data.test_name) {
            pb.finish_and_clear();
            self.multi_progress.remove(&pb);
        }
        self.ticker_tx
            .send(TickerEvent::Remove { id })
            .expect("ticker disconnected");

        self.multi_progress
            .suspend(|| self.formatter.fmt_test_outcome(data.for_formatter))
    }

    fn fmt_run_outcomes(&mut self, data: Self::RunOutcomes) -> Result<(), Self::Error> {
        self.ticker
            .take()
            .map(|join_handle| join_handle.join())
            .transpose()
            .expect("could not join ticker");
        self.multi_progress.clear()?;

        self.multi_progress
            .suspend(|| self.formatter.fmt_run_outcomes(data))
    }
}

impl<'t> GroupedTestFormatter<'t, Extra, GroupKey, GroupCtx> for ProgressFormatter<'t> {
    type GroupedRunStart = <InnerFormatter<'t> as GroupedTestFormatter<
        't,
        Extra,
        GroupKey,
        GroupCtx,
    >>::GroupedRunStart;
    type GroupStart =
        <InnerFormatter<'t> as GroupedTestFormatter<'t, Extra, GroupKey, GroupCtx>>::GroupStart;
    type GroupOutcomes =
        <InnerFormatter<'t> as GroupedTestFormatter<'t, Extra, GroupKey, GroupCtx>>::GroupOutcomes;
    type GroupedRunOutcomes = <InnerFormatter<'t> as GroupedTestFormatter<
        't,
        Extra,
        GroupKey,
        GroupCtx,
    >>::GroupedRunOutcomes;

    fn fmt_grouped_run_start(&mut self, data: Self::GroupedRunStart) -> Result<(), Self::Error> {
        <InnerFormatter<'t> as GroupedTestFormatter<'t, Extra, GroupKey, GroupCtx>>::fmt_grouped_run_start(
            &mut self.formatter,
            data,
        )
    }

    fn fmt_group_start(&mut self, data: Self::GroupStart) -> Result<(), Self::Error> {
        <InnerFormatter<'t> as GroupedTestFormatter<'t, Extra, GroupKey, GroupCtx>>::fmt_group_start(
            &mut self.formatter,
            data,
        )
    }

    fn fmt_group_outcomes(&mut self, data: Self::GroupOutcomes) -> Result<(), Self::Error> {
        <InnerFormatter<'t> as GroupedTestFormatter<'t, Extra, GroupKey, GroupCtx>>::fmt_group_outcomes(
            &mut self.formatter,
            data,
        )
    }

    fn fmt_grouped_run_outcomes(
        &mut self,
        data: Self::GroupedRunOutcomes,
    ) -> Result<(), Self::Error> {
        <InnerFormatter<'t> as GroupedTestFormatter<'t, Extra, GroupKey, GroupCtx>>::fmt_grouped_run_outcomes(
            &mut self.formatter,
            data,
        )
    }
}
