//! archaven-driven architecture test. Declarative module-boundary rules
//! that mirror `tests/architecture.rs`. The hand-rolled test stays as a
//! fast smoke check; this one is the canonical, declarative version.
//!
//! Run with `cargo test --test archaven` or `make archaven`.

use archaven::{Access, Archaven, Rule};

#[test]
fn module_boundaries() {
    let archaven = Archaven::new()
        // Domain must not depend on infra/mcp/web.
        .rule(
            Rule::new()
                .named("domain purity")
                .deny(
                    Access::from("crate::domain::*")
                        .to("crate::infra::*")
                        .because("domain must not depend on infra"),
                )
                .deny(
                    Access::from("crate::domain::*")
                        .to("crate::mcp::*")
                        .because("domain must not depend on mcp"),
                )
                .deny(
                    Access::from("crate::domain::*")
                        .to("crate::web::*")
                        .because("domain must not depend on web"),
                ),
        )
        // Infra must not depend on mcp or web.
        .rule(
            Rule::new()
                .named("infra isolation")
                .deny(
                    Access::from("crate::infra::*")
                        .to("crate::mcp::*")
                        .because("infra must not depend on mcp"),
                )
                .deny(
                    Access::from("crate::infra::*")
                        .to("crate::web::*")
                        .because("infra must not depend on web"),
                ),
        )
        // MCP tools must use use-cases, not repos directly.
        .rule(
            Rule::new().named("mcp via use cases").deny(
                Access::from("crate::mcp::tools_impl::*")
                    .to("crate::infra::repos::*")
                    .because("mcp tools must call use-cases, not repos directly"),
            ),
        );

    let violations = archaven.check("./src").expect("archaven check should run");
    violations.assert_empty();
}
