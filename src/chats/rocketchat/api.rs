use super::schema::*;
use reqwest::blocking::Client;
use url::Url;

pub struct RocketChatApi {
    client: Client,
    host: Url,
    username: String,
    password: String,
    auth_token: Option<AuthToken>,
}

impl RocketChatApi {
    pub fn new(host: Url, username: String, password: String) -> Self {
        let client = Client::new();
        RocketChatApi {
            client,
            host,
            username,
            password,
            auth_token: None,
        }
    }

    pub fn login(&mut self) -> Result<(), String> {
        let login_response = self
            .client
            .post(&format!("{}/api/v1/login", &self.host)[..])
            .body(format!(
                "username={}&password={}",
                &self.username, &self.password
            ))
            .header("content-type", "application/x-www-form-urlencoded")
            .send()
            .map_err(|err| format!("{:?}", err))?
            .json::<LoginResponse>()
            .map_err(|err| format!("{:?}", err))?;
        self.auth_token = Some(login_response.data);
        Ok(())
    }

    pub fn channels(&self) -> Result<Vec<ChannelResponse>, String> {
        let channels = self
            .client
            .get(&format!("{}/api/v1/channels.list", &self.host)[..])
            .header(
                "X-Auth-Token",
                &self.auth_token.as_ref().unwrap().auth_token,
            )
            .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
            .send()
            .map_err(|err| format!("{:?}", err))?
            .json::<ChannelListResponse>()
            .map_err(|err| format!("{:?}", err))?
            .channels;
        Ok(channels)
    }

    pub fn users(&self) -> Result<Vec<UserResponse>, String> {
        let users = self
            .client
            .get(&format!("{}/api/v1/users.list", &self.host)[..])
            .header(
                "X-Auth-Token",
                &self.auth_token.as_ref().unwrap().auth_token,
            )
            .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
            .send()
            .map_err(|err| format!("{:?}", err))?
            .json::<UserListResponse>()
            .map_err(|err| format!("{:?}", err))?
            .users;
        Ok(users)
    }
    pub fn history(&self, room_id: String, count: usize) -> Result<Vec<MessageResponse>, String> {
        let messages = self
            .client
            .get(
                &format!(
                    "{}/api/v1/channels.history?roomId={}&count={}",
                    &self.host, room_id, count
                )[..],
            )
            .header(
                "X-Auth-Token",
                &self.auth_token.as_ref().unwrap().auth_token,
            )
            .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
            .send()
            .map_err(|err| format!("{:?}", err))?
            .json::<ChannelHistoryResponse>()
            .map_err(|err| format!("{:?}", err))?
            .messages;
        Ok(messages)
    }

    pub fn send_message(&self, room_id: String, content: String) -> Result<(), String> {
        self.client
            .post(&format!("{}/api/v1/chat.postMessage", &self.host)[..])
            .body(format!(
                "{{ \"channel\": \"{}\", \"text\": \"{}\" }}",
                room_id, content
            ))
            .header(
                "X-Auth-Token",
                &self.auth_token.as_ref().unwrap().auth_token,
            )
            .header("X-User-Id", &self.auth_token.as_ref().unwrap().user_id)
            .header("content-type", "application/json")
            .send()
            .map_err(|err| format!("{:?}", err))?;
        Ok(())
    }
}
