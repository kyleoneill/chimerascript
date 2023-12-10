from flask import Flask, request, make_response, json

app = Flask("simple-server")

@app.route("/")
def entry():
    return '{"status":"online","nest":{"test":5}}'

@app.route("/test_resource", methods=["GET","PUT","POST","DELETE"])
def test_resource():
    example = {
        "name": "example_resource",
        "location": "my_computer",
        "endpoints": 2,
        "has_values": True
    }
    if request.method == "GET":
        first_query_param = request.args.get("first", None)
        second_query_param = request.args.get("second", None)
        if first_query_param != None or second_query_param != None:
            return json.jsonify({"resource":example, "extras":{"first":first_query_param, "second":second_query_param}})
        else:
            return json.jsonify(example)
    elif request.method == "PUT":
        json_data = request.get_json()
        for thing in json_data:
            if thing in example:
                example[thing] = json_data[thing]
            else:
                return make_response(json.jsonify({"error":"bad body param"}), 400)
        return json.jsonify(example)
    elif request.method == "POST":
        json_data = request.get_json()
        res = {}
        for key in example:
            if key not in json_data:
                return make_response(json.jsonify({"error":f"missing field {key}"}), 400)
            res[key] = json_data[key]
        return make_response(json.jsonify(res), 201)
    elif request.method == "DELETE":
        return make_response("", 200)
    else:
        return make_response('{"error":"unsupported method"}', 400)
