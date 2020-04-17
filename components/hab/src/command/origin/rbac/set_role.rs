use crate::{api_client::{self,
                         Client},
            common::ui::{Status,
                         UIReader,
                         UIWriter,
                         UI},
            error::{Error,
                    Result},
            PRODUCT,
            VERSION};
use reqwest::StatusCode;

pub async fn start(ui: &mut UI,
                   bldr_url: &str,
                   origin: &str,
                   token: &str,
                   member_account: &str,
                   role: &str,
                   no_prompt: bool)
                   -> Result<()> {
    let api_client = Client::new(bldr_url, PRODUCT, VERSION, None).map_err(Error::APIClient)?;

    ui.begin(format!("Preparing to update member {}'s role to '{}' in origin {}",
                     member_account, role, origin))?;

    if !no_prompt {
        if !confirm_update_role(ui)? {
            return Ok(());
        };
    }

    match api_client.update_member_role(origin, token, member_account, role)
                    .await
    {
        Ok(_) => {
            ui.status(Status::Updated, "the member role successfully!".to_string())
              .or(Ok(()))
        }
        Err(err @ api_client::Error::APIError(StatusCode::FORBIDDEN, _)) => {
            ui.fatal("Failed to update the role!")?;
            ui.fatal("This situation could arise, if for example, you are not a member with \
                      sufficient privileges in the  origin.")?;
            Err(Error::APIClient(err))
        }
        Err(err @ api_client::Error::APIError(StatusCode::NOT_FOUND, _)) => {
            ui.fatal("Failed to update the role!")?;
            ui.fatal("This situation could arise, if for example, you passed a invalid member \
                      or origin name.")?;
            Err(Error::APIClient(err))
        }
        Err(e) => {
            ui.fatal(format!("Failed to update the role! {:?}", e))?;
            Err(Error::from(e))
        }
    }
}

fn confirm_update_role(ui: &mut UI) -> Result<bool> {
    Ok(ui.prompt_yes_no("Modify the role as indicated above?", Some(true))?)
}
