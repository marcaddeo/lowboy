use fake::faker::company::en::CompanyName;
use fake::faker::internet::en::SafeEmail;
use fake::faker::job::en::Title;
use fake::faker::name::en::{FirstName, LastName};
use fake::Fake;

pub struct User {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub byline: String,
    pub avatar: String,
}

impl User {
    pub fn fake() -> Self {
        let first_name: String = FirstName().fake();
        let last_name: String = LastName().fake();

        let email: String = SafeEmail().fake();

        let byline = format!(
            "{} - {}",
            Title().fake::<String>(),
            CompanyName().fake::<String>()
        );

        let avatar = format!(
            "https://avatar.iran.liara.run/username?username={}+{}",
            first_name, last_name
        );

        Self {
            first_name,
            last_name,
            email,
            byline,
            avatar,
        }
    }

    pub fn current() -> Self {
        Self {
            first_name: "Marc".to_string(),
            last_name: "Addeo".to_string(),
            email: "hi@marc.cx".to_string(),
            byline: "Super cool guy".to_string(),
            avatar: "https://avatars.githubusercontent.com/u/199649?v=4".to_string(),
        }
    }
}
