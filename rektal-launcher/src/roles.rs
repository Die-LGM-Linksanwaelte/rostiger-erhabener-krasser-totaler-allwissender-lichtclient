/// this enum defines the roles that can be assigned to a user
/// The roles are:
/// - Programmer
/// - blind Programmer
/// - Operator
/// - Interface
pub enum Roles {
    Programmer,
    BlindProgrammer,
    Operator,
    Interface,
}

///Implementation of method as_str to get the Role Name as a &str
impl Roles {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Roles::Programmer => "Programmer",
            Roles::BlindProgrammer => "blind Programmer",
            Roles::Operator => "Operator",
            Roles::Interface => "Interface",
        }
    }
}