#!/usr/bin/env python3
"""Resolve and render the arbitrary-N static RDP route-user inventory."""

from __future__ import annotations

import argparse
import json
import os
import secrets
import string
import sys
from pathlib import Path

CANONICAL_ENV = "AGENT_BROWSER_RDP_ROUTE_USER_POOL_JSON"
CANONICAL_SECRET = "XRDP_AGENT_BROWSER_ROUTE_USER_POOL_JSON"
PASSWORD_ALPHABET = string.ascii_letters + string.digits + "-_."
MAX_CONNECTIONS = 8
MAX_CONNECTIONS_PER_USER = 8


def read_env_file(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    if not path.is_file():
        return values
    for line in path.read_text().splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or "=" not in stripped:
            continue
        key, value = stripped.split("=", 1)
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
            value = value[1:-1]
        values[key.strip()] = value
    return values


def legacy_inventory(values: dict[str, str]) -> list[dict[str, str]]:
    routes = []
    for label, default_user in (("A", "agent-browser-rdp-a"), ("B", "agent-browser-rdp-b")):
        routes.append(
            {
                "id": f"legacy-route-{label.lower()}",
                "connectionName": os.environ.get(
                    f"AGENT_BROWSER_RDP_ROUTE_{label}_CONNECTION_NAME",
                    f"Agent Browser RDP Route {label}",
                ),
                "legacyConnectionName": os.environ.get(
                    f"AGENT_BROWSER_RDP_ROUTE_{label}_LEGACY_CONNECTION_NAME",
                    f"Agent Browser RDP Existing User Route {label}",
                ),
                "routeUser": os.environ.get(f"AGENT_BROWSER_RDP_ROUTE_{label}_USERNAME")
                or values.get(f"XRDP_AGENT_BROWSER_ROUTE_{label}_USERNAME")
                or default_user,
                "password": os.environ.get(f"AGENT_BROWSER_RDP_ROUTE_{label}_PASSWORD")
                or values.get(f"XRDP_AGENT_BROWSER_ROUTE_{label}_PASSWORD")
                or "",
            }
        )
    return routes


def normalized_inventory(raw: object) -> list[dict[str, str]]:
    if not isinstance(raw, list) or len(raw) < 2:
        raise ValueError("route_user_inventory_requires_at_least_two_entries")
    routes: list[dict[str, str]] = []
    for index, item in enumerate(raw):
        if not isinstance(item, dict):
            raise ValueError(f"route_user_inventory_entry_invalid:{index}")
        route = {
            "id": text(item.get("id")) or f"route-user-{index + 1}",
            "connectionName": text(item.get("connectionName")) or "",
            "legacyConnectionName": text(item.get("legacyConnectionName")) or "",
            "routeUser": text(item.get("routeUser")) or "",
            "password": text(item.get("password")) or "",
        }
        for field in ("id", "connectionName", "routeUser"):
            if not route[field]:
                raise ValueError(f"route_user_inventory_{field}_missing:{index}")
        if any("\n" in value or "\r" in value or "\t" in value for value in route.values()):
            raise ValueError(f"route_user_inventory_control_character:{index}")
        routes.append(route)
    require_unique(routes, "id")
    require_unique(routes, "connectionName")
    require_unique(routes, "routeUser")
    return routes


def resolve_inventory(
    secret_file: Path, generate_passwords: bool, allow_missing_passwords: bool = False
) -> list[dict[str, str]]:
    values = read_env_file(secret_file)
    raw_text = os.environ.get(CANONICAL_ENV) or values.get(CANONICAL_SECRET)
    raw = json.loads(raw_text) if raw_text else legacy_inventory(values)
    routes = normalized_inventory(raw)
    if generate_passwords:
        for route in routes:
            if not route["password"]:
                route["password"] = "".join(secrets.choice(PASSWORD_ALPHABET) for _ in range(32))
        write_inventory_secret(secret_file, routes)
    elif not allow_missing_passwords and any(not route["password"] for route in routes):
        raise ValueError("route_user_inventory_password_missing")
    return routes


def write_inventory_secret(path: Path, routes: list[dict[str, str]]) -> None:
    removable = {
        CANONICAL_SECRET,
        "XRDP_AGENT_BROWSER_ROUTE_A_USERNAME",
        "XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD",
        "XRDP_AGENT_BROWSER_ROUTE_B_USERNAME",
        "XRDP_AGENT_BROWSER_ROUTE_B_PASSWORD",
    }
    existing = path.read_text().splitlines() if path.exists() else []
    retained = [
        line
        for line in existing
        if line.split("=", 1)[0].strip() not in removable
    ]
    retained.append(f"{CANONICAL_SECRET}={json.dumps(routes, separators=(',', ':'))}")
    for label, route in zip(("A", "B"), routes[:2], strict=True):
        retained.append(f"XRDP_AGENT_BROWSER_ROUTE_{label}_USERNAME={route['routeUser']}")
        retained.append(f"XRDP_AGENT_BROWSER_ROUTE_{label}_PASSWORD={route['password']}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(retained) + "\n")
    path.chmod(0o600)


def render_sql(
    routes: list[dict[str, str]],
    hostname: str,
    port: str,
    max_connections: int = MAX_CONNECTIONS,
    max_connections_per_user: int = MAX_CONNECTIONS_PER_USER,
) -> str:
    declarations = ["  canonical_count integer;", "  legacy_count integer;"]
    declarations.extend(f"  route_id_{index} integer;" for index in range(len(routes)))
    declarations.extend(
        [
            "  final_canonical_count integer;",
            "  final_legacy_count integer;",
            "  distinct_username_count integer;",
        ]
    )
    blocks = [
        route_sql_block(
            route,
            index,
            hostname,
            port,
            max_connections,
            max_connections_per_user,
        )
        for index, route in enumerate(routes)
    ]
    canonical_names = ", ".join(quote(route["connectionName"]) for route in routes)
    legacy_names = [route["legacyConnectionName"] for route in routes if route["legacyConnectionName"]]
    legacy_name_sql = ", ".join(quote(name) for name in legacy_names) or "NULL"
    route_ids = ", ".join(f"route_id_{index}" for index in range(len(routes)))
    return f"""BEGIN;

DO $$
DECLARE
{chr(10).join(declarations)}
BEGIN
{chr(10).join(blocks)}

  SELECT count(*) INTO final_canonical_count
  FROM guacamole_connection
  WHERE parent_id IS NULL AND connection_name IN ({canonical_names});

  SELECT count(*) INTO final_legacy_count
  FROM guacamole_connection
  WHERE parent_id IS NULL AND connection_name IN ({legacy_name_sql});

  SELECT count(DISTINCT parameter_value) INTO distinct_username_count
  FROM guacamole_connection_parameter
  WHERE connection_id IN ({route_ids}) AND parameter_name = 'username';

  IF final_canonical_count <> {len(routes)}
     OR final_legacy_count <> 0
     OR distinct_username_count <> {len(routes)} THEN
    RAISE EXCEPTION
      'route-user inventory postcondition failed: canonical %, legacy %, distinct usernames %',
      final_canonical_count, final_legacy_count, distinct_username_count;
  END IF;
END $$;

COMMIT;"""


def route_sql_block(
    route: dict[str, str],
    index: int,
    hostname: str,
    port: str,
    max_connections: int,
    max_connections_per_user: int,
) -> str:
    route_id = f"route_id_{index}"
    canonical = quote(route["connectionName"])
    legacy = quote(route["legacyConnectionName"]) if route["legacyConnectionName"] else "NULL"
    names = f"{canonical}, {legacy}" if route["legacyConnectionName"] else canonical
    params = {
        "hostname": hostname,
        "port": port,
        "username": route["routeUser"],
        "password": route["password"],
        "security": "any",
        "ignore-cert": "true",
        "resize-method": "display-update",
        "enable-audio-input": "false",
        "enable-drive": "false",
        "enable-theming": "false",
        "enable-wallpaper": "false",
    }
    values = ",\n".join(
        f"    ({route_id}, {quote(name)}, {quote(value)})" for name, value in params.items()
    )
    legacy_update = f"""
  IF legacy_count = 1 THEN
    UPDATE guacamole_connection
    SET connection_name = {canonical}, protocol = 'rdp', max_connections = {max_connections},
        max_connections_per_user = {max_connections_per_user}
    WHERE parent_id IS NULL AND connection_name = {legacy}
    RETURNING connection_id INTO {route_id};
  ELSIF canonical_count = 1 THEN""" if route["legacyConnectionName"] else """
  IF canonical_count = 1 THEN"""
    return f"""
  SELECT count(*) FILTER (WHERE connection_name = {canonical}),
         count(*) FILTER (WHERE connection_name = {legacy})
  INTO canonical_count, legacy_count
  FROM guacamole_connection
  WHERE parent_id IS NULL AND connection_name IN ({names});

  IF canonical_count + legacy_count > 1 THEN
    RAISE EXCEPTION 'ambiguous managed route {index + 1}';
  END IF;
{legacy_update}
    UPDATE guacamole_connection
    SET protocol = 'rdp', max_connections = {max_connections},
        max_connections_per_user = {max_connections_per_user}
    WHERE parent_id IS NULL AND connection_name = {canonical}
    RETURNING connection_id INTO {route_id};
  ELSE
    INSERT INTO guacamole_connection (
      connection_name, protocol, max_connections, max_connections_per_user
    ) VALUES ({canonical}, 'rdp', {max_connections}, {max_connections_per_user})
    RETURNING connection_id INTO {route_id};
  END IF;

  DELETE FROM guacamole_connection_parameter
  WHERE connection_id = {route_id} AND parameter_name = 'color-depth';

  INSERT INTO guacamole_connection_parameter (
    connection_id, parameter_name, parameter_value
  ) VALUES
{values}
  ON CONFLICT (connection_id, parameter_name) DO UPDATE
  SET parameter_value = EXCLUDED.parameter_value;

  INSERT INTO guacamole_connection_permission (entity_id, connection_id, permission)
  SELECT entity.entity_id, {route_id}, 'READ'::guacamole_object_permission_type
  FROM guacamole_entity entity WHERE entity.type = 'USER'
  ON CONFLICT DO NOTHING;
"""


def require_unique(routes: list[dict[str, str]], field: str) -> None:
    values = [route[field] for route in routes]
    if len(set(values)) != len(values):
        raise ValueError(f"route_user_inventory_duplicate_{field}")


def quote(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


def text(value: object) -> str | None:
    return value.strip() if isinstance(value, str) and value.strip() else None


def bounded_connection_limit(value: str) -> int:
    parsed = int(value)
    if parsed < 1 or parsed > 64:
        raise argparse.ArgumentTypeError("connection limit must be between 1 and 64")
    return parsed


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    resolve = subparsers.add_parser("resolve")
    resolve.add_argument("--secret-file", required=True)
    resolve.add_argument("--generate-passwords", action="store_true")
    resolve.add_argument("--allow-missing-passwords", action="store_true")
    sql = subparsers.add_parser("sql")
    sql.add_argument("--hostname", required=True)
    sql.add_argument("--port", required=True)
    sql.add_argument("--max-connections", type=bounded_connection_limit, default=MAX_CONNECTIONS)
    sql.add_argument(
        "--max-connections-per-user",
        type=bounded_connection_limit,
        default=MAX_CONNECTIONS_PER_USER,
    )
    args = parser.parse_args()
    try:
        if args.command == "resolve":
            routes = resolve_inventory(
                Path(args.secret_file),
                args.generate_passwords,
                args.allow_missing_passwords,
            )
            print(json.dumps(routes, separators=(",", ":")))
        else:
            routes = normalized_inventory(json.load(sys.stdin))
            if any(not route["password"] for route in routes):
                raise ValueError("route_user_inventory_password_missing")
            print(
                render_sql(
                    routes,
                    args.hostname,
                    args.port,
                    args.max_connections,
                    args.max_connections_per_user,
                )
            )
        return 0
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
