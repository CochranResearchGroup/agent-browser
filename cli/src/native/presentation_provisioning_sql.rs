pub(crate) struct RouteProvisionSqlInput<'a> {
    pub connection_name: &'a str,
    pub route_user: &'a str,
    pub password: &'a str,
    pub sharing_profile_name: &'a str,
    pub hostname: &'a str,
    pub port: u16,
    pub max_connections: u32,
    pub max_connections_per_user: u32,
}

pub(crate) fn render_route_provision_sql(
    input: &RouteProvisionSqlInput<'_>,
) -> Result<String, String> {
    let texts = [
        input.connection_name,
        input.route_user,
        input.password,
        input.sharing_profile_name,
        input.hostname,
    ];
    if texts
        .iter()
        .any(|value| value.is_empty() || value.contains('\0'))
        || input.port == 0
        || !(1..=64).contains(&input.max_connections)
        || !(1..=64).contains(&input.max_connections_per_user)
    {
        return Err("presentation_route_provision_invalid_input".to_string());
    }

    let connection_name = sql_literal(input.connection_name);
    let route_user = sql_literal(input.route_user);
    let password = sql_literal(input.password);
    let sharing_profile_name = sql_literal(input.sharing_profile_name);
    let hostname = sql_literal(input.hostname);
    let port = sql_literal(&input.port.to_string());
    let max_connections = input.max_connections;
    let max_connections_per_user = input.max_connections_per_user;
    let mut tag_sequence = 0_u32;
    let block_tag = loop {
        let candidate = format!("$agent_browser_route_provision_{tag_sequence}$");
        if texts.iter().all(|value| !value.contains(&candidate)) {
            break candidate;
        }
        tag_sequence = tag_sequence
            .checked_add(1)
            .ok_or_else(|| "presentation_route_provision_invalid_input".to_string())?;
    };
    let parameters = [
        ("hostname", hostname.as_str()),
        ("port", port.as_str()),
        ("username", route_user.as_str()),
        ("password", password.as_str()),
        ("security", "E'any'"),
        ("ignore-cert", "E'true'"),
        ("resize-method", "E'display-update'"),
        ("enable-audio-input", "E'false'"),
        ("enable-drive", "E'false'"),
        ("enable-theming", "E'false'"),
        ("enable-wallpaper", "E'false'"),
    ];
    let parameter_values = parameters
        .iter()
        .map(|(name, value)| format!("    (route_connection_id, E'{name}', {value})"))
        .collect::<Vec<_>>()
        .join(",\n");
    let expected_parameters = parameters
        .iter()
        .map(|(name, value)| format!("(E'{name}', {value})"))
        .collect::<Vec<_>>()
        .join(", ");

    Ok(format!(
        r#"BEGIN;

LOCK TABLE guacamole_connection,
  guacamole_connection_parameter,
  guacamole_connection_permission,
  guacamole_sharing_profile,
  guacamole_sharing_profile_parameter,
  guacamole_sharing_profile_permission,
  guacamole_entity IN SHARE ROW EXCLUSIVE MODE;

DO {block_tag}
DECLARE
  canonical_count bigint;
  route_connection_id integer;
  route_sharing_profile_id integer;
  expected_parameter_count integer := {parameter_count};
  actual_parameter_count bigint;
  matching_parameter_count bigint;
  user_count bigint;
BEGIN
  SELECT count(*), min(connection_id)
  INTO canonical_count, route_connection_id
  FROM guacamole_connection
  WHERE connection_name = {connection_name};

  IF canonical_count > 1 THEN
    RAISE EXCEPTION 'presentation_route_provision_connection_drift';
  END IF;

  IF EXISTS (
    SELECT 1
    FROM guacamole_connection_parameter parameter
    WHERE parameter.parameter_name = 'username'
      AND parameter.parameter_value = {route_user}
      AND (route_connection_id IS NULL OR parameter.connection_id <> route_connection_id)
  ) THEN
    RAISE EXCEPTION 'presentation_route_provision_route_user_conflict';
  END IF;

  IF canonical_count = 0 THEN
    IF EXISTS (
      SELECT 1 FROM guacamole_sharing_profile
      WHERE sharing_profile_name = {sharing_profile_name}
    ) THEN
      RAISE EXCEPTION 'presentation_route_provision_sharing_profile_conflict';
    END IF;

    INSERT INTO guacamole_connection (
      connection_name, protocol, max_connections, max_connections_per_user
    ) VALUES (
      {connection_name}, 'rdp', {max_connections}, {max_connections_per_user}
    ) RETURNING connection_id INTO route_connection_id;

    INSERT INTO guacamole_connection_parameter (
      connection_id, parameter_name, parameter_value
    ) VALUES
{parameter_values};

    INSERT INTO guacamole_connection_permission (entity_id, connection_id, permission)
    SELECT entity_id, route_connection_id, 'READ'::guacamole_object_permission_type
    FROM guacamole_entity WHERE type = 'USER';

    INSERT INTO guacamole_sharing_profile (
      sharing_profile_name, primary_connection_id
    ) VALUES (
      {sharing_profile_name}, route_connection_id
    ) RETURNING sharing_profile_id INTO route_sharing_profile_id;

    INSERT INTO guacamole_sharing_profile_parameter (
      sharing_profile_id, parameter_name, parameter_value
    ) VALUES (route_sharing_profile_id, 'read-only', 'false');

    INSERT INTO guacamole_sharing_profile_permission (
      entity_id, sharing_profile_id, permission
    )
    SELECT entity_id, route_sharing_profile_id, 'READ'::guacamole_object_permission_type
    FROM guacamole_entity WHERE type = 'USER';
  ELSE
    IF NOT EXISTS (
      SELECT 1 FROM guacamole_connection
      WHERE connection_id = route_connection_id
        AND parent_id IS NULL
        AND protocol = 'rdp'
        AND max_connections = {max_connections}
        AND max_connections_per_user = {max_connections_per_user}
    ) THEN
      RAISE EXCEPTION 'presentation_route_provision_connection_drift';
    END IF;

    SELECT count(*), count(*) FILTER (
      WHERE (parameter_name, parameter_value) IN ({expected_parameters})
    )
    INTO actual_parameter_count, matching_parameter_count
    FROM guacamole_connection_parameter
    WHERE connection_id = route_connection_id;
    IF actual_parameter_count <> expected_parameter_count
       OR matching_parameter_count <> expected_parameter_count THEN
      RAISE EXCEPTION 'presentation_route_provision_parameter_drift';
    END IF;

    SELECT min(sharing_profile_id), count(*)
    INTO route_sharing_profile_id, canonical_count
    FROM guacamole_sharing_profile
    WHERE sharing_profile_name = {sharing_profile_name}
      AND primary_connection_id = route_connection_id;
    IF canonical_count <> 1
       OR EXISTS (
         SELECT 1 FROM guacamole_sharing_profile
         WHERE sharing_profile_name = {sharing_profile_name}
           AND primary_connection_id <> route_connection_id
       )
       OR (SELECT count(*) FROM guacamole_sharing_profile_parameter
           WHERE sharing_profile_id = route_sharing_profile_id) <> 1
       OR NOT EXISTS (
         SELECT 1 FROM guacamole_sharing_profile_parameter
         WHERE sharing_profile_id = route_sharing_profile_id
           AND parameter_name = 'read-only' AND parameter_value = 'false'
       )
       OR (SELECT count(*) FROM guacamole_sharing_profile
           WHERE primary_connection_id = route_connection_id) <> 1
       THEN
      RAISE EXCEPTION 'presentation_route_provision_sharing_profile_drift';
    END IF;

    SELECT count(*) INTO user_count FROM guacamole_entity WHERE type = 'USER';
    IF (SELECT count(*) FROM guacamole_connection_permission
        WHERE connection_id = route_connection_id
          AND permission = 'READ'::guacamole_object_permission_type) <> user_count
       OR EXISTS (
         SELECT 1 FROM guacamole_entity entity
         WHERE entity.type = 'USER' AND NOT EXISTS (
           SELECT 1 FROM guacamole_connection_permission permission
           WHERE permission.entity_id = entity.entity_id
             AND permission.connection_id = route_connection_id
             AND permission.permission = 'READ'::guacamole_object_permission_type
         )
       )
       OR EXISTS (
         SELECT 1 FROM guacamole_connection_permission
         WHERE connection_id = route_connection_id
           AND permission <> 'READ'::guacamole_object_permission_type
       )
       OR (SELECT count(*) FROM guacamole_sharing_profile_permission
           WHERE sharing_profile_id = route_sharing_profile_id
             AND permission = 'READ'::guacamole_object_permission_type) <> user_count
       OR EXISTS (
         SELECT 1 FROM guacamole_entity entity
         WHERE entity.type = 'USER' AND NOT EXISTS (
           SELECT 1 FROM guacamole_sharing_profile_permission permission
           WHERE permission.entity_id = entity.entity_id
             AND permission.sharing_profile_id = route_sharing_profile_id
             AND permission.permission = 'READ'::guacamole_object_permission_type
         )
       )
       OR EXISTS (
         SELECT 1 FROM guacamole_sharing_profile_permission
         WHERE sharing_profile_id = route_sharing_profile_id
           AND permission <> 'READ'::guacamole_object_permission_type
       ) THEN
      RAISE EXCEPTION 'presentation_route_provision_permission_drift';
    END IF;
  END IF;
END;
{block_tag};

SELECT json_build_object(
  'connectionId', connection.connection_id,
  'connectionName', connection.connection_name,
  'routeUser', parameter.parameter_value
) AS route_provision
FROM guacamole_connection connection
JOIN guacamole_connection_parameter parameter
  ON parameter.connection_id = connection.connection_id
 AND parameter.parameter_name = 'username'
WHERE connection.connection_name = {connection_name};

COMMIT;
"#,
        parameter_count = parameters.len(),
    ))
}

fn sql_literal(value: &str) -> String {
    format!("E'{}'", value.replace('\\', "\\\\").replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input<'a>(password: &'a str) -> RouteProvisionSqlInput<'a> {
        RouteProvisionSqlInput {
            connection_name: "Route O'Brien\\East",
            route_user: "route-user",
            password,
            sharing_profile_name: "Shared Route",
            hostname: "rdp.internal",
            port: 3389,
            max_connections: 8,
            max_connections_per_user: 8,
        }
    }

    #[test]
    fn quotes_sql_literals_and_backslashes() {
        let sql = render_route_provision_sql(&input("p'ass\\word")).unwrap();
        assert!(sql.contains("E'Route O''Brien\\\\East'"));
        assert!(sql.contains("E'p''ass\\\\word'"));
    }

    #[test]
    fn renders_insert_only_exact_replay_contract() {
        let sql = render_route_provision_sql(&input("secret-value")).unwrap();
        assert!(sql.contains("LOCK TABLE guacamole_connection"));
        assert!(sql.contains("INSERT INTO guacamole_connection"));
        assert_eq!(sql.matches("INSERT INTO guacamole_connection (").count(), 1);
        assert!(sql.contains("presentation_route_provision_parameter_drift"));
        assert!(sql.contains("json_build_object("));
        assert!(!sql.contains("UPDATE "));
        assert!(!sql.contains("DELETE "));
        assert!(!sql.contains("ON CONFLICT DO UPDATE"));
    }

    #[test]
    fn uses_fixed_redacted_errors() {
        let sql = render_route_provision_sql(&input("never-in-an-error")).unwrap();
        for line in sql.lines().filter(|line| line.contains("RAISE EXCEPTION")) {
            assert!(!line.contains("never-in-an-error"));
        }
        assert_eq!(
            render_route_provision_sql(&input("bad\0secret")),
            Err("presentation_route_provision_invalid_input".to_string())
        );
    }

    #[test]
    fn chooses_a_dollar_quote_tag_absent_from_inputs() {
        let sql = render_route_provision_sql(&input("$agent_browser_route_provision_0$")).unwrap();
        assert!(sql.contains("DO $agent_browser_route_provision_1$"));
        assert!(sql.contains("\n$agent_browser_route_provision_1$;"));
    }

    #[test]
    fn rejects_zero_or_unbounded_limits() {
        let mut invalid = input("secret");
        invalid.max_connections = 65;
        assert_eq!(
            render_route_provision_sql(&invalid),
            Err("presentation_route_provision_invalid_input".to_string())
        );
        invalid.max_connections = 1;
        invalid.port = 0;
        assert!(render_route_provision_sql(&invalid).is_err());
    }
}
