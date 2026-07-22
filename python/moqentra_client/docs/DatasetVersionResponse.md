# DatasetVersionResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** |  | 
**dataset_id** | **str** |  | 
**state** | **str** |  | 

## Example

```python
from moqentra_client.models.dataset_version_response import DatasetVersionResponse

# TODO update the JSON string below
json = "{}"
# create an instance of DatasetVersionResponse from a JSON string
dataset_version_response_instance = DatasetVersionResponse.from_json(json)
# print the JSON string representation of the object
print(DatasetVersionResponse.to_json())

# convert the object into a dict
dataset_version_response_dict = dataset_version_response_instance.to_dict()
# create an instance of DatasetVersionResponse from a dict
dataset_version_response_from_dict = DatasetVersionResponse.from_dict(dataset_version_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


