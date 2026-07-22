# CompileResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**digest** | **str** |  | 
**edge_count** | **int** |  | 
**node_count** | **int** |  | 
**capabilities** | **List[str]** |  | 

## Example

```python
from moqentra_client.models.compile_response import CompileResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CompileResponse from a JSON string
compile_response_instance = CompileResponse.from_json(json)
# print the JSON string representation of the object
print(CompileResponse.to_json())

# convert the object into a dict
compile_response_dict = compile_response_instance.to_dict()
# create an instance of CompileResponse from a dict
compile_response_from_dict = CompileResponse.from_dict(compile_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


