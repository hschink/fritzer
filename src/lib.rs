use ring::{digest, pbkdf2};
use std::num::NonZeroU32;
use url::Url;

pub mod command;
pub mod connection;

use crate::command::{Device, FritzboxLogin, FritzboxSwitchOperator, Login, SwitchOperator};
use crate::connection::SessionInfo;

static INVALID_SESSION: &str = "0000000000000000";
const CREDENTIAL_LEN: usize = digest::SHA256_OUTPUT_LEN;
type Credential = [u8; CREDENTIAL_LEN];

pub struct Fritzbox<
    L: Login + ?Sized = FritzboxLogin,
    S: SwitchOperator + ?Sized = FritzboxSwitchOperator,
> {
    pub session_info: Option<SessionInfo>,

    url: Url,
    client: reqwest::Client,
    login: Box<L>,
    switch_operator: Box<S>,
}

impl Fritzbox<FritzboxLogin, FritzboxSwitchOperator> {
    pub fn new(url: Url) -> Fritzbox<FritzboxLogin, FritzboxSwitchOperator> {
        Fritzbox {
            session_info: None::<SessionInfo>,

            url,
            client: reqwest::Client::new(),
            login: Box::new(FritzboxLogin),
            switch_operator: Box::new(FritzboxSwitchOperator),
        }
    }
}

impl<L, S> Fritzbox<L, S>
where
    L: Login,
    S: SwitchOperator,
{
    pub fn with_login(url: Url, login: L) -> Fritzbox<L, FritzboxSwitchOperator> {
        Fritzbox {
            session_info: None::<SessionInfo>,

            url,
            client: reqwest::Client::new(),
            login: Box::new(login),
            switch_operator: Box::new(FritzboxSwitchOperator {}),
        }
    }

    pub fn with_switchbox_operator(url: Url, login: L, switch_operator: S) -> Fritzbox<L, S> {
        Fritzbox {
            session_info: None::<SessionInfo>,

            url,
            client: reqwest::Client::new(),
            login: Box::new(login),
            switch_operator: Box::new(switch_operator),
        }
    }

    pub fn is_connected(&self) -> bool {
        match &self.session_info {
            Some(s) => !s.sid.eq(INVALID_SESSION),
            None => false,
        }
    }

    pub async fn update_session_info(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.session_info = self.login.get_session_info(&self.client, &self.url).await?;

        Ok(())
    }

    pub async fn connect_with_sid(
        &mut self,
        sid: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        self.session_info = self
            .login
            .connect_with_sid(&self.client, &self.url, sid)
            .await?;

        Ok(self.is_connected())
    }

    pub async fn connect_with_credentials(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        self.update_session_info().await?;

        let session_info = self.session_info.as_ref().unwrap();
        let response = Self::get_challenge_response(&session_info.challenge, password);

        self.session_info = self
            .login
            .connect_with_credentials(&self.client, &self.url, username, &response)
            .await?;

        Ok(self.is_connected())
    }

    pub async fn get_switches(&self) -> Result<Vec<Device>, Box<dyn std::error::Error>> {
        let session_info = self.session_info.as_ref().unwrap();

        Ok(self
            .switch_operator
            .get_switches(&self.client, &self.url, &session_info.sid)
            .await?)
    }

    fn get_challenge_response(challenge: &str, password: &str) -> String {
        let challenges: Vec<&str> = challenge.split('$').collect();
        let salt1 = hex::decode(challenges[2]).unwrap();
        let salt2 = hex::decode(challenges[4]).unwrap();

        let mut hash1: Credential = [0u8; CREDENTIAL_LEN];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(challenges[1].parse::<u32>().unwrap()).unwrap(),
            &salt1,
            password.as_bytes(),
            &mut hash1,
        );

        let mut hash2: Credential = [0u8; CREDENTIAL_LEN];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(challenges[3].parse::<u32>().unwrap()).unwrap(),
            &salt2,
            &hash1,
            &mut hash2,
        );

        format!("{}%24{}", challenges[4], hex::encode(hash2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use std::error::Error;

    use crate::connection::{User, Users};

    #[test]
    fn fritzbox_is_connected_should_return_false_by_default() {
        // Arrange
        let url = Url::parse("http://localhost").expect("No valid URL.");
        let fritzbox = Fritzbox::new(url);

        // Act

        // Assert
        assert_eq!(false, fritzbox.is_connected());
    }

    #[tokio::test]
    async fn fritzbox_is_connected_should_return_true_on_valid_sid() {
        // Arrange
        let url = Url::parse("http://localhost").expect("No valid URL.");
        let session_info = SessionInfo {
            sid: "1".repeat(16),
            challenge: String::new(),
            users: Users {
                users: Vec::<User>::new(),
            },
        };
        let login = MockFritzboxLogin::with_session_info(&Some(session_info));
        let mut fritzbox = Fritzbox::<MockFritzboxLogin>::with_login(url, login);

        // Act
        let _ = fritzbox.update_session_info().await;

        // Assert
        assert_eq!(true, fritzbox.is_connected());
    }

    #[tokio::test]
    async fn fritzbox_is_connected_should_return_false_on_invalid_sid() {
        // Arrange
        let url = Url::parse("http://localhost").expect("No valid URL.");
        let session_info = SessionInfo {
            sid: INVALID_SESSION.to_string(),
            challenge: String::new(),
            users: Users {
                users: Vec::<User>::new(),
            },
        };
        let login = MockFritzboxLogin::with_session_info(&Some(session_info));
        let mut fritzbox =
            Fritzbox::<MockFritzboxLogin, MockFritzboxSwitchOperator>::with_login(url, login);

        // Act
        let _ = fritzbox.update_session_info().await;

        // Assert
        assert_eq!(false, fritzbox.is_connected());
    }

    #[tokio::test]
    async fn fritzbox_get_switches_should_return_devices() {
        // Arrange
        let url = Url::parse("http://localhost").expect("No valid URL.");
        let session_info = SessionInfo {
            sid: "1".repeat(16),
            challenge: String::new(),
            users: Users {
                users: Vec::<User>::new(),
            },
        };
        let switches = vec![
            Device {
                ain: "000001".to_string(),
                name: "test1".to_string(),
            },
            Device {
                ain: "000002".to_string(),
                name: "test2".to_string(),
            },
        ];
        let login = MockFritzboxLogin::with_session_info(&Some(session_info));
        let switch_operator = MockFritzboxSwitchOperator::with_switches(switches);
        let mut fritzbox =
            Fritzbox::<MockFritzboxLogin, MockFritzboxSwitchOperator>::with_switchbox_operator(
                url,
                login,
                switch_operator,
            );

        let _ = fritzbox.update_session_info().await;

        // Act
        let result = fritzbox.get_switches().await.unwrap();

        // Assert
        assert_eq!(2, result.len());
    }

    #[test]
    fn fritzbox_get_challenge_response_should_return_valid_response() {
        // Arrange
        let challenge =
            "2$60000$c5b7ff41801c5f877d307bbdc93188ef$6000$d19cee81917f97da37430f45b8352db0";
        let password = "my$uper$trongPa$$w0rd4U";

        // Act
        let response = Fritzbox::<FritzboxLogin>::get_challenge_response(&challenge, &password);

        // Assert
        assert_eq!("d19cee81917f97da37430f45b8352db0%24506cf2017a1f3ff399bd66d750979ebdb0cc22fbdaa134acf2ad26c71df6c20f", response);
    }

    impl Clone for SessionInfo {
        fn clone(&self) -> Self {
            Self {
                sid: self.sid.clone(),
                challenge: self.challenge.clone(),
                users: Users {
                    users: Vec::<User>::new(),
                },
            }
        }
    }

    pub struct MockFritzboxLogin {
        session_info: Option<SessionInfo>,
    }

    impl MockFritzboxLogin {
        fn with_session_info(session_info: &Option<SessionInfo>) -> MockFritzboxLogin {
            MockFritzboxLogin {
                session_info: session_info.clone(),
            }
        }
    }

    #[async_trait]
    impl Login for MockFritzboxLogin {
        async fn get_session_info(
            &self,
            _client: &reqwest::Client,
            _url: &Url,
        ) -> Result<Option<SessionInfo>, Box<dyn Error>> {
            Ok(self.session_info.clone())
        }

        async fn connect_with_sid(
            &self,
            _client: &reqwest::Client,
            _url: &Url,
            _sid: &str,
        ) -> Result<Option<SessionInfo>, Box<dyn Error>> {
            Ok(self.session_info.clone())
        }

        async fn connect_with_credentials(
            &self,
            _client: &reqwest::Client,
            _url: &Url,
            _username: &str,
            _response: &str,
        ) -> Result<Option<SessionInfo>, Box<dyn Error>> {
            Ok(self.session_info.clone())
        }
    }

    pub struct MockFritzboxSwitchOperator {
        switches: Vec<Device>,
    }

    impl MockFritzboxSwitchOperator {
        fn with_switches(switches: Vec<Device>) -> MockFritzboxSwitchOperator {
            MockFritzboxSwitchOperator { switches }
        }
    }

    #[async_trait]
    impl SwitchOperator for MockFritzboxSwitchOperator {
        async fn get_switches(
            &self,
            _client: &reqwest::Client,
            _url: &Url,
            _sid: &str,
        ) -> Result<Vec<Device>, Box<dyn Error>> {
            Ok(self.switches.clone())
        }
    }
}
