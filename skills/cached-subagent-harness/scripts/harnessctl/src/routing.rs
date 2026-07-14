use crate::domain::{Profile, Risk, Role, RouteDecision, RouteDemand};

pub(crate) fn required_profile(demand: &RouteDemand) -> Profile {
    let risk_floor = match demand.risk {
        Risk::Low => Profile::Light,
        Risk::Medium => Profile::Standard,
        Risk::High | Risk::Critical => Profile::Deep,
    };
    let role_floor = match demand.role {
        Role::Discussion | Role::Explorer => Profile::Light,
        Role::Worker | Role::Fixer => Profile::Standard,
        Role::Reviewer => Profile::Deep,
    };
    demand
        .complexity
        .max(risk_floor)
        .max(role_floor)
        .max(demand.uncertainty)
}

pub(crate) fn route(demand: &RouteDemand, manual: Option<Profile>) -> RouteDecision {
    let floor = required_profile(demand);
    let mut reason_codes = vec![format!("safety_floor_{}", floor.as_str())];
    let (profile, manual_lowering_rejected) = match manual {
        Some(requested) if requested < floor => {
            reason_codes.push("manual_lowering_rejected".into());
            (floor, true)
        }
        Some(requested) if requested > floor => {
            reason_codes.push("manual_elevation".into());
            (requested, false)
        }
        _ => (floor, false),
    };
    RouteDecision {
        profile,
        reason_codes,
        manual_lowering_rejected,
    }
}

#[cfg(test)]
mod tests {
    use super::{required_profile, route};
    use crate::domain::{Profile, Risk, Role, RouteDemand};

    #[test]
    fn routing_uses_the_strongest_safety_floor() {
        let light = RouteDemand {
            complexity: Profile::Light,
            risk: Risk::Low,
            role: Role::Explorer,
            uncertainty: Profile::Light,
        };
        assert_eq!(required_profile(&light), Profile::Light);

        let standard = RouteDemand {
            complexity: Profile::Light,
            risk: Risk::Medium,
            role: Role::Worker,
            uncertainty: Profile::Light,
        };
        assert_eq!(required_profile(&standard), Profile::Standard);

        let deep = RouteDemand {
            complexity: Profile::Standard,
            risk: Risk::Critical,
            role: Role::Reviewer,
            uncertainty: Profile::Standard,
        };
        assert_eq!(required_profile(&deep), Profile::Deep);
    }

    #[test]
    fn manual_profile_can_raise_but_never_lower_the_floor() {
        let standard = RouteDemand {
            complexity: Profile::Standard,
            risk: Risk::Low,
            role: Role::Worker,
            uncertainty: Profile::Light,
        };
        assert_eq!(route(&standard, Some(Profile::Deep)).profile, Profile::Deep);

        let deep = RouteDemand {
            complexity: Profile::Deep,
            ..standard
        };
        let decision = route(&deep, Some(Profile::Light));
        assert_eq!(decision.profile, Profile::Deep);
        assert!(decision.manual_lowering_rejected);
        assert!(
            decision
                .reason_codes
                .contains(&"manual_lowering_rejected".into())
        );
    }
}
