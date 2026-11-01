"""The device subset of sf.execution/2.0-draft11 (Python standard library)."""
import json
from urllib.parse import urlsplit

VERSION = "sf.execution/2.0-draft11"
MAX_UINT = 9007199254740991


class ProtocolError(ValueError):
    """A fixed, non-secret diagnostic; never include received bodies or URLs."""


def encoded(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":"), allow_nan=False).encode("utf-8")


def fields(value, required):
    if not isinstance(value, dict) or set(value) != set(required.split()):
        raise ProtocolError("invalid object fields")


def identifier(value):
    if not isinstance(value, str) or not value or "\0" in value:
        raise ProtocolError("invalid identifier")


def uint(value, maximum=MAX_UINT):
    if type(value) is not int or not 0 <= value <= maximum:
        raise ProtocolError("invalid unsigned integer")


def boolean(value):
    if type(value) is not bool:
        raise ProtocolError("invalid boolean")


def identity(value):
    fields(value, "session_id execution_epoch_id project_id host_context_session_id")
    for v in value.values():
        identifier(v)


def command(value, config):
    fields(value, "contract_version identity attempt device_id action parameters target_device_boot_id")
    if (value["contract_version"] != VERSION or value["action"] not in ("process_unit", "prepare_equipment", "transport_cargo")
            or value["identity"] != config["identity"]
            or value["device_id"] != config["device_id"]
            or value["target_device_boot_id"] != config["device_boot_id"]):
        raise ProtocolError("command identity or capability mismatch")
    attempt = value["attempt"]
    fields(attempt, "logical_job_id phase_id attempt_ordinal command_id selected_binding_id phase_token")
    for k, v in attempt.items():
        uint(v) if k == "attempt_ordinal" else identifier(v)
    params = value["parameters"]
    fields(params, "quantity phase_id nominal_work_ms" + (" setup_transition" if "setup_transition" in params else ""))
    if "setup_transition" in params:
        fields(params["setup_transition"], "from_family to_family")
        identifier(params["setup_transition"]["to_family"])
        if params["setup_transition"]["from_family"] is not None:
            identifier(params["setup_transition"]["from_family"])
        if value["action"] != "prepare_equipment" or params["phase_id"] != "setup":
            raise ProtocolError("setup requires an equipment preparation command")
    if params["phase_id"] != attempt["phase_id"]:
        raise ProtocolError("command phase mismatch")
    uint(params["nominal_work_ms"])
    fields(params["quantity"], "value unit")
    identifier(params["quantity"]["unit"])
    amount = params["quantity"]["value"]
    fields(amount, "coefficient decimal_places")
    uint(amount["coefficient"])
    uint(amount["decimal_places"], 9)


def probe(value, config):
    fields(value, "probe_id device_id device_boot_id issued_execution_revision issued_at_host_ms expires_at_host_ms")
    identifier(value["probe_id"])
    if (value["device_id"] != config["device_id"]
            or value["device_boot_id"] != config["device_boot_id"]):
        raise ProtocolError("probe identity mismatch")
    for k in ["issued_execution_revision", "issued_at_host_ms", "expires_at_host_ms"]:
        uint(value[k])
    # Host owns expiry/admission. The device never substitutes its local clock.


def poll_result(value, request, config):
    fields(value, "poll_request_id state command observation_probe")
    if value["poll_request_id"] != request["poll_request_id"]:
        raise ProtocolError("poll identity mismatch")
    if value["state"] == "empty":
        if value["command"] is not None:
            raise ProtocolError("empty poll contains command")
    elif value["state"] == "command" and request["accept_commands"]:
        command(value["command"], config)
    else:
        raise ProtocolError("unexpected poll outcome")
    if value["observation_probe"] is not None:
        probe(value["observation_probe"], config)


def bootstrap(value):
    """Private stdin envelope, not an additional cloud wire contract."""
    fields(value, "contract_version mode base_url token journal_path device_id device_boot_id dataset_id identity processing_time_ms accept_commands" + (" processing_time_source" if "processing_time_source" in value else "") + (" processing_speed" if "processing_speed" in value else ""))
    if "processing_time_source" in value and (value["processing_time_source"] != "command" or value["mode"] != "execution"):
        raise ProtocolError("command timing requires execution mode")
    if "processing_speed" in value:
        uint(value["processing_speed"])
        if value.get("processing_time_source") != "command" or not 1 <= value["processing_speed"] <= 30:
            raise ProtocolError("demonstration speed requires command timing and must be 1..30")
    if value["contract_version"] != VERSION or value["mode"] not in ("execution", "telemetry_only"):
        raise ProtocolError("unsupported bootstrap")
    for k in ["token", "journal_path", "device_id", "device_boot_id", "dataset_id"]:
        identifier(value[k])
    if "\r" in value["token"] or "\n" in value["token"]:
        raise ProtocolError("invalid credential")
    if not isinstance(value["base_url"], str):
        raise ProtocolError("invalid service endpoint")
    parsed = urlsplit(value["base_url"])
    if (parsed.scheme != "https" or not parsed.hostname or parsed.username
            or parsed.password or parsed.query or parsed.fragment
            or parsed.path not in ("", "/")):
        raise ProtocolError("service endpoint must be a fixed HTTPS origin")
    uint(value["processing_time_ms"])
    boolean(value["accept_commands"])
    if value["mode"] == "execution":
        identity(value["identity"])
    elif value["identity"] is not None or value["accept_commands"]:
        raise ProtocolError("telemetry-only cannot own execution")
    return value


def response(value, expected_kind):
    if not isinstance(value, dict) or value.get("contract_version") != VERSION:
        raise ProtocolError("unsupported service response")
    if "error" in value:
        # Do not echo an untrusted server message (it can contain credentials).
        raise ProtocolError("service rejected request")
    fields(value, "contract_version result")
    fields(value["result"], "kind data")
    if value["result"]["kind"] != expected_kind:
        raise ProtocolError("unexpected service response kind")
    return value["result"]["data"]
