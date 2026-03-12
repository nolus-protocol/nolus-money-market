use crate::{AccessPermission, error::Error, user::User};

/// An *OR* composite permission
///
/// It is granted to an user if either of the individual permissions grants access.
/// This permission allows infinite compositions of permissions.
pub struct Permission<'main_permission, 'fallback_permission, MainPermission, FallbackPermission> {
    main: &'main_permission MainPermission,
    fallback: &'fallback_permission FallbackPermission,
}

impl<'main_permission, 'fallback_permission, MainPermission, FallbackPermission>
    Permission<'main_permission, 'fallback_permission, MainPermission, FallbackPermission>
{
    pub fn new(
        main: &'main_permission MainPermission,
        fallback: &'fallback_permission FallbackPermission,
    ) -> Self {
        Self { main, fallback }
    }
}

impl<'main_permission, 'fallback_permission, MainPermission, FallbackPermission> AccessPermission
    for Permission<'main_permission, 'fallback_permission, MainPermission, FallbackPermission>
where
    MainPermission: AccessPermission,
    FallbackPermission: AccessPermission,
{
    fn granted_to<U>(&self, user: &U) -> Result<bool, Error>
    where
        U: User,
    {
        match self.main.granted_to(user) {
            Ok(true) => Ok(true),
            _ => self.fallback.granted_to(user),
        }
    }
}
