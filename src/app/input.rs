use super::{App, ItemFormField, LoginField, Screen, Tab, VaultMode};

impl App {
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        match &mut self.screen {
            Screen::ServerConfig { url, busy, .. } => {
                if *busy {
                    return;
                }
                match key.code {
                    KeyCode::Esc => self.should_quit = true,
                    KeyCode::Enter => self.confirm_server_config(),
                    KeyCode::Backspace => {
                        url.pop();
                    }
                    KeyCode::Char(c) => url.push(c),
                    _ => {}
                }
            }
            Screen::Login {
                email,
                password,
                focus,
                awaiting_2fa,
                code,
                method,
                error,
                busy,
            } => {
                if *busy {
                    return;
                }
                if *awaiting_2fa {
                    match key.code {
                        KeyCode::Esc => self.should_quit = true,
                        KeyCode::Tab => *method = method.toggled(),
                        KeyCode::Enter => {
                            *error = None;
                            self.try_login();
                        }
                        KeyCode::Backspace => {
                            code.pop();
                        }
                        KeyCode::Char(c) => code.push(c),
                        _ => {}
                    }
                    return;
                }
                match key.code {
                    KeyCode::Esc => self.should_quit = true,
                    KeyCode::Tab => {
                        *focus = match focus {
                            LoginField::Email => LoginField::Password,
                            LoginField::Password => LoginField::Email,
                        }
                    }
                    KeyCode::Enter => match focus {
                        LoginField::Email => *focus = LoginField::Password,
                        LoginField::Password => {
                            *error = None;
                            self.try_login();
                        }
                    },
                    KeyCode::Backspace => match focus {
                        LoginField::Email => {
                            email.pop();
                        }
                        LoginField::Password => {
                            password.pop();
                        }
                    },
                    KeyCode::Char(c) => match focus {
                        LoginField::Email => email.push(c),
                        LoginField::Password => password.push(c),
                    },
                    _ => {}
                }
            }
            Screen::Unlock {
                password,
                error,
                busy,
                ..
            } => {
                if *busy {
                    return;
                }
                match key.code {
                    KeyCode::Esc => self.should_quit = true,
                    KeyCode::Enter => {
                        *error = None;
                        self.try_unlock();
                    }
                    KeyCode::Backspace => {
                        password.pop();
                    }
                    KeyCode::Char(c) => password.push(c),
                    _ => {}
                }
            }
            Screen::Loading => {}
            Screen::Main => self.handle_main_key(key),
        }
    }

    fn handle_main_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        if self.item_form.is_none() {
            match key.code {
                KeyCode::Tab => {
                    self.tab = self.tab.next();
                    return;
                }
                KeyCode::BackTab => {
                    self.tab = self.tab.prev();
                    return;
                }
                _ => {}
            }
        }

        match self.tab {
            Tab::Vault => match self.vault_mode {
                VaultMode::Search => match key.code {
                    KeyCode::Esc => {
                        self.query.clear();
                        self.refilter();
                        self.vault_mode = VaultMode::Normal;
                    }
                    KeyCode::Enter => self.vault_mode = VaultMode::Normal,
                    KeyCode::Backspace => {
                        self.query.pop();
                        self.refilter();
                    }
                    KeyCode::Char(c) => {
                        self.query.push(c);
                        self.refilter();
                    }
                    _ => {}
                },
                VaultMode::Normal if self.item_form.is_some() => {
                    if self.item_form.as_ref().unwrap().generator_open {
                        match key.code {
                            KeyCode::Esc => self.close_item_form_password_picker(),
                            KeyCode::Char('g') => self.generate_password(),
                            KeyCode::Enter => self.confirm_item_form_password_picker(),
                            code => self.apply_generator_option_key(code),
                        }
                        return;
                    }
                    let ctrl_char = |c| key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char(c);
                    if ctrl_char('g') {
                        self.open_item_form_password_picker();
                        return;
                    }
                    if ctrl_char('r') {
                        self.reveal_current_password_in_item_form();
                        return;
                    }
                    if ctrl_char('t') {
                        self.cycle_item_kind(1);
                        return;
                    }
                    match key.code {
                        KeyCode::Esc => {
                            self.item_form = None;
                            return;
                        }
                        KeyCode::Enter => {
                            self.submit_item_form();
                            return;
                        }
                        _ => {}
                    }
                    let form = self.item_form.as_mut().unwrap();
                    match key.code {
                        KeyCode::Tab => form.cycle_focus(1),
                        KeyCode::BackTab => form.cycle_focus(-1),
                        KeyCode::Backspace => {
                            if form.focus == ItemFormField::Password {
                                form.password_revealed = false;
                            }
                            form.focused_field_mut().pop();
                        }
                        KeyCode::Char(c) => {
                            if form.focus == ItemFormField::Password {
                                form.password_revealed = false;
                            }
                            let focus = form.focus;
                            let field = form.focused_field_mut();
                            let within_limit = match focus.max_len() {
                                Some(max) => field.chars().count() < max,
                                None => true,
                            };
                            if within_limit {
                                field.push(c);
                            }
                        }
                        _ => {}
                    }
                }
                VaultMode::Normal if self.detail_open => match key.code {
                    KeyCode::Esc => self.detail_open = false,
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Enter => {
                        self.detail_open = false;
                        self.copy_primary_secret();
                    }
                    KeyCode::Char('u') => self.copy_username(),
                    KeyCode::Char('t') => self.copy_totp(),
                    KeyCode::Char('r') => self.toggle_reveal(),
                    KeyCode::Char('n') => self.copy_notes(),
                    KeyCode::Char('e') => self.open_edit_form(),
                    _ => {}
                },
                VaultMode::Normal => {
                    if key.code != KeyCode::Char('g') {
                        self.pending_g = false;
                    }
                    match key.code {
                        KeyCode::Esc => {
                            if !self.query.is_empty() {
                                self.query.clear();
                                self.refilter();
                            } else {
                                self.should_quit = true;
                            }
                        }
                        KeyCode::Char('q') => self.should_quit = true,
                        KeyCode::Char('/') => self.vault_mode = VaultMode::Search,
                        KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
                        KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
                        KeyCode::Char('g') => {
                            if self.pending_g {
                                self.pending_g = false;
                                self.selected = 0;
                                self.reveal = None;
                                self.reveal_cvv = None;
                            } else {
                                self.pending_g = true;
                            }
                        }
                        KeyCode::Char('G') => {
                            self.selected = self.filtered.len().saturating_sub(1);
                            self.reveal = None;
                            self.reveal_cvv = None;
                        }
                        KeyCode::Char('h') | KeyCode::Left => self.cycle_folder(-1),
                        KeyCode::Char('l') | KeyCode::Right => self.cycle_folder(1),
                        KeyCode::Char('n') => self.open_create_form(),
                        KeyCode::Enter => {
                            if self.selected_item().is_some() {
                                self.detail_open = true;
                            }
                        }
                        KeyCode::Char('R') | KeyCode::F(5) => self.refresh_items(),
                        _ => {}
                    }
                }
            },
            Tab::Generator => match key.code {
                KeyCode::Esc => self.should_quit = true,
                KeyCode::Enter => self.generate_password(),
                KeyCode::Char('c') => self.copy_generated(),
                code => self.apply_generator_option_key(code),
            },
            Tab::Account => {
                if self.confirm_logout {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => self.logout_now(),
                        _ => self.confirm_logout = false,
                    }
                    return;
                }
                match key.code {
                    KeyCode::Esc => self.should_quit = true,
                    KeyCode::Char('s') => self.sync_now(),
                    KeyCode::Char('l') => self.lock_now(),
                    KeyCode::Char('o') => self.confirm_logout = true,
                    _ => {}
                }
            }
        }
    }
}
