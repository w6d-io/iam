package app.rbac

import rego.v1

default main := false

main if {
	some i

	# set the role  if the ressource id is present in the role attribute of the user
	roles = data.metadata_public[_]
	roles[i].key == input.resource
	role := roles[i].value
	match_url({
		"method": input.method,
		"uri": input.uri,
		"role": role,
	})
}

#transform uri query to base query
uri_pipeline_stream := "api/mux/pipeline" if {
	regex.match(`api\/iam\/pipeline\/stream\?projectId=([0-9])+&eventId+=([A-z])+`, input.uri)
} else := input.uri

uri_pipeline_events := "api/iam/pipeline" if {
	regex.match(`api\/iam\/pipeline\/events\?projectId=([0-9])+&eventId+=([A-z])+(:?&perPage=([0-9])+)?(:?&page=([0-9])+)?`, uri_pipeline_stream)
} else := uri_pipeline_stream

uri_notif_stream := "api/iam/notif" if {
	regex.match(`api\/iam\/notif\/stream\?projectId=([0-9])+&eventId+=([A-z])+`, uri_pipeline_stream)
} else := uri_pipeline_events

uri_notif_events := "api/iam/notif" if {
	regex.match(`api\/iam\/notif\/events\?projectId=([0-9])+&eventId+=([A-z])+(:?&perPage=([0-9])+)?(:?&page=([0-9])+)?`, uri_notif_stream)
} else := uri_notif_stream

match_url(var) if {
	some k
	api_attributes = {"get": [
		{"key": "api/iam/pipeline", "value": ["admin", "owner", "billing", "editor", "contributor"]},
		{"key": "api/iam/notif", "value": ["admin", "owner", "billing", "editor", "contributor"]},
	]}

	uri_list := api_attributes[var.method]
	uri_list[k].key == uri_notif_stream
	uri_list[k].value[_] == var.role
}
