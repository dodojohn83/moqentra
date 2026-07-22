# GenerateSplitsRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**seed** | **int** |  | 
**train** | **float** |  | 
**val** | **float** |  | 
**test** | **float** |  | 

## Example

```python
from moqentra_client.models.generate_splits_request import GenerateSplitsRequest

# TODO update the JSON string below
json = "{}"
# create an instance of GenerateSplitsRequest from a JSON string
generate_splits_request_instance = GenerateSplitsRequest.from_json(json)
# print the JSON string representation of the object
print(GenerateSplitsRequest.to_json())

# convert the object into a dict
generate_splits_request_dict = generate_splits_request_instance.to_dict()
# create an instance of GenerateSplitsRequest from a dict
generate_splits_request_from_dict = GenerateSplitsRequest.from_dict(generate_splits_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


