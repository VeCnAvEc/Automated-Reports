use crate::r#type::types::ResponseError;

#[derive(Debug)]
pub struct Settings {
    prod: bool,
}

impl Settings {
    pub fn new() -> Self {
        Settings { prod: false }
    }

    pub fn set_prod(&mut self, arg: Vec<String>) -> Result<(), ResponseError> {
        let args = ArgReqeust::check_for_starting_arguments(arg);

        if let Err(error) = ArgReqeust::activate_tasks_sent_via_terminal(self, args) {
            return Err(error);
        }

        Ok(())
    }

    pub fn get_prod(&self) -> bool {
        self.prod
    }
}

#[derive(Debug)]
pub struct ArgReqeust {
    pub key: String,
    pub val: String,
}

type TerminalArguments = Vec<ArgReqeust>;

// @57432
impl ArgReqeust {
    pub fn check_for_starting_arguments(arguments: Vec<String>) -> TerminalArguments {
        let mut commands: TerminalArguments = Vec::new();

        for argument in arguments {
            let key_value = argument.split("=").collect::<Vec<&str>>();
            if key_value.len() == 2 {
                commands.push(ArgReqeust {
                    key: key_value[0].to_string(),
                    val: key_value[1].to_string(),
                })
            }
        }

        commands
    }

    pub fn activate_tasks_sent_via_terminal(
        settings: &mut Settings,
        arguments: TerminalArguments,
    ) -> Result<(), ResponseError> {
        let mut errors: Vec<ResponseError> = Vec::new();
        for arg in arguments {
            match arg.key.to_lowercase().as_str() {
                "proda" => match arg.val.to_lowercase().as_str() {
                    "true" => {
                        settings.prod = true;
                    }
                    "false" => {
                        settings.prod = false;
                    }
                    _ => {
                        errors.push((
                            1574320,
                            "У нас нету возможности обработать такое значение".to_string(),
                        ));
                    }
                },
                _ => errors.push((5435432, "Не известный ключ.".to_string())),
            }
        }
        if !errors.is_empty() {
            return Err(errors.last().unwrap().clone());
        }
        Ok(())
    }
}
