import os, sys, json

import caolo_web_lib as cw

from flask import Blueprint, request, jsonify, abort
from flask_login import login_required, current_user
from twisted.python import log

from ..model.program import Program
from ..model import db
from ..model.command import Command

from ..service import get_redis_client

import input_messages_pb2
import scripts_pb2

script_bp = Blueprint("script", __name__, url_prefix="/script")


@script_bp.errorhandler(400)
def resource_not_found(e):
    return jsonify(error=str(e)), 400


@script_bp.route('/compile', methods=["POST"])
def compile_script():
    content = request.get_data(as_text=True)
    try:
        _program = cw.compile(content)
        return "successful compilation"
    except ValueError as e:
        log.err()
        abort(400, e)


@script_bp.route('/commit', methods=["POST"])
@login_required
def commit_script():
    content = request.get_data(as_text=True)
    try:
        compiled = cw.compile(content)
    except ValueError as e:
        log.err()
        abort(400, e)

    program = json.loads(content)

    try:
        name = program.pop('name')
    except KeyError:
        log.err()
        abort(400, "name was not set")

    program = Program(
        program=program,
        compiled=compiled,
        user=current_user,
        name=name,
    )

    db.session.add(program)
    db.session.commit()

    msg = input_messages_pb2.InputMsg()
    msg.update_script.user_id = current_user.id.encode('utf8')
    msg.update_script.script_id = program.id.encode('utf8')
    for k, v in compiled['labels'].items():
        msg.update_script.compiled_script.labels[k].block = v[0]
        msg.update_script.compiled_script.labels[k].myself = v[1]

    command = Command(
        user=current_user, raw_payload=msg.update_script.SerializeToString())

    db.session.add(command)
    db.session.commit()

    msg.msg_id = command.id.encode('utf8')

    content = msg.SerializeToString()

    redis_conn = get_redis_client()
    redis_conn.lpush("INPUTS", content)

    return program.id


@script_bp.route('/schema', methods=["GET"])
def get_schema():
    schema = cw.get_basic_schema()
    redis_conn = get_redis_client()
    payload = redis_conn.get("SCHEMA")
    if payload:
        schema.extend(json.loads(payload))
    return jsonify(schema)


@script_bp.route('/my_scripts', methods=["GET"])
@login_required
def get_my_scripts():
    query = Program.query.filter_by(user_id=current_user.id)
    result = list({
        "id": row.id,
        "program": row.program,
        "name": row.name
    } for row in query)
    return jsonify(result)
