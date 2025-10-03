use std::{borrow::Cow, collections::HashMap, marker::PhantomData, ops::Deref};

use nu_experimental::ExperimentalOption;
use nu_path::AbsolutePathBuf;
use nu_protocol::{
    PipelineData, ShellError,
    engine::{EngineState, Stack},
};

fn test() -> NuTestBuilder<
    HasShellCommandContext<false>,
    HasExtraCommandContext<false>,
    HasPluginCommandContext<false>,
> {
    NuTestBuilder {
        cwd: Default::default(),
        locale: Default::default(),
        envs: Default::default(),
        experimental_options: Default::default(),
        _marker: (
            HasShellCommandContext,
            HasExtraCommandContext,
            HasPluginCommandContext,
        ),
    }
}

struct HasShellCommandContext<const B: bool>;
struct HasExtraCommandContext<const B: bool>;
struct HasPluginCommandContext<const B: bool>;

struct NuTestBuilder<HasShellCommandContext, HasExtraCommandContext, HasPluginCommandContext> {
    cwd: Option<AbsolutePathBuf>,
    locale: Option<Cow<'static, str>>,
    envs: Option<HashMap<String, String>>,
    experimental_options: Option<Vec<(&'static ExperimentalOption, bool)>>,
    _marker: (
        HasShellCommandContext,
        HasExtraCommandContext,
        HasPluginCommandContext,
    ),
}

impl<const S: bool, const E: bool, const P: bool> NuTestBuilder<HasShellCommandContext<S>, HasExtraCommandContext<E>, HasPluginCommandContext<P>> {
    pub fn cwd(self, cwd: impl Into<AbsolutePathBuf>) -> Self {
        Self {
            cwd: Some(cwd.into()),
            ..self
        }
    }

    pub fn locale(self, locale: impl Into<Cow<'static, str>>) -> Self {
        Self {
            locale: Some(locale.into()),
            ..self
        }
    }

    pub fn env(self, key: impl ToString, value: impl ToString) -> Self {
        let mut envs = self.envs.unwrap_or_default();
        envs.insert(key.to_string(), value.to_string());
        Self {
            envs: Some(envs),
            ..self
        }
    }

    pub fn envs(self, envs: impl IntoIterator<Item = (String, String)>) -> Self {
        let mut current_envs = self.envs.unwrap_or_default();
        current_envs.extend(envs);
        Self {
            envs: Some(current_envs),
            ..self
        }
    }

    pub fn experimental_option(self, option: &'static ExperimentalOption, enable: bool) -> Self {
        let mut options = self.experimental_options.unwrap_or_default();
        options.push((option, enable));
        Self {
            experimental_options: Some(options),
            ..self
        }
    }

    pub fn experimental_options(
        self,
        options: impl IntoIterator<Item = (&'static ExperimentalOption, bool)>,
    ) -> Self {
        let mut current_options = self.experimental_options.unwrap_or_default();
        current_options.extend(options);
        Self {
            experimental_options: Some(current_options),
            ..self
        }
    }

    pub fn execute(self, code: &str) -> Result<NuTestExecutor, NuTestError> {
        let engine_state = nu_cmd_lang::create_default_context();
        let engine_state = match S {
            true => nu_command::add_shell_command_context(engine_state),
            false => engine_state,
        };
        
        todo!()
    }
}

impl<const E: bool, const P: bool>
    NuTestBuilder<
        HasShellCommandContext<false>,
        HasExtraCommandContext<E>,
        HasPluginCommandContext<P>,
    >
{
    pub fn add_shell_command_context(
        self,
    ) -> NuTestBuilder<
        HasShellCommandContext<true>,
        HasExtraCommandContext<E>,
        HasPluginCommandContext<P>,
    > {
        self.cast()
    }
}

impl<const S: bool, const P: bool>
    NuTestBuilder<
        HasShellCommandContext<S>,
        HasExtraCommandContext<false>,
        HasPluginCommandContext<P>,
    >
{
    pub fn add_extra_command_context(
        self,
    ) -> NuTestBuilder<
        HasShellCommandContext<S>,
        HasExtraCommandContext<true>,
        HasPluginCommandContext<P>,
    > {
        self.cast()
    }
}

impl<const S: bool, const E: bool>
    NuTestBuilder<
        HasShellCommandContext<S>,
        HasExtraCommandContext<E>,
        HasPluginCommandContext<false>,
    >
{
    pub fn add_plugin_command_context(
        self,
    ) -> NuTestBuilder<
        HasShellCommandContext<S>,
        HasExtraCommandContext<E>,
        HasPluginCommandContext<true>,
    > {
        self.cast()
    }
}

impl<const SI: bool, const EI: bool, const PI: bool>
    NuTestBuilder<
        HasShellCommandContext<SI>,
        HasExtraCommandContext<EI>,
        HasPluginCommandContext<PI>,
    >
{
    fn cast<const SO: bool, const EO: bool, const PO: bool>(
        self,
    ) -> NuTestBuilder<
        HasShellCommandContext<SO>,
        HasExtraCommandContext<EO>,
        HasPluginCommandContext<PO>,
    > {
        let NuTestBuilder {
            cwd,
            locale,
            envs,
            experimental_options,
            _marker,
        } = self;
        NuTestBuilder {
            cwd,
            locale,
            envs,
            experimental_options,
            _marker: (
                HasShellCommandContext,
                HasExtraCommandContext,
                HasPluginCommandContext,
            ),
        }
    }
}

struct NuTestExecutor {
    pub engine_state: EngineState,
    pub stack: Stack,
    pub last_pipeline: PipelineData,
}

struct NuTestError {
    pub engine_state: EngineState,
    pub stack: Stack,
    pub error: ShellError,
}

impl std::fmt::Debug for NuTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

impl NuTestExecutor {
    pub fn execute(self, code: &str) -> Result<NuTestExecutor, NuTestError> {
        todo!()
    }
}

fn some_test() {
    test()
        .locale("de")
        .execute("first command")
        .unwrap()
        .execute("second command")
        .unwrap()
        .execute("third command")
        .unwrap();
}
