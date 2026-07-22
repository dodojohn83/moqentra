# ReadyResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**status** | **str** |  | 
**ready** | **bool** |  | 

## Example

```python
from moqentra_client.models.ready_response import ReadyResponse

# TODO update the JSON string below
json = "{}"
# create an instance of ReadyResponse from a JSON string
ready_response_instance = ReadyResponse.from_json(json)
# print the JSON string representation of the object
print(ReadyResponse.to_json())

# convert the object into a dict
ready_response_dict = ready_response_instance.to_dict()
# create an instance of ReadyResponse from a dict
ready_response_from_dict = ReadyResponse.from_dict(ready_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


