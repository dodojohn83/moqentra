# AutosaveRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**client_update_id** | **str** |  | 
**payload** | **object** |  | 

## Example

```python
from moqentra_client.models.autosave_request import AutosaveRequest

# TODO update the JSON string below
json = "{}"
# create an instance of AutosaveRequest from a JSON string
autosave_request_instance = AutosaveRequest.from_json(json)
# print the JSON string representation of the object
print(AutosaveRequest.to_json())

# convert the object into a dict
autosave_request_dict = autosave_request_instance.to_dict()
# create an instance of AutosaveRequest from a dict
autosave_request_from_dict = AutosaveRequest.from_dict(autosave_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


