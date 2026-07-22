# ImportJobResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** |  | 
**state** | **str** |  | 
**source_url** | **str** |  | 
**target_key** | **str** |  | 
**media_type** | **str** |  | 
**total_bytes** | **int** |  | 
**transferred_bytes** | **int** |  | 
**concurrency** | **int** |  | 
**deadline_seconds** | **int** |  | 
**digest** | **str** |  | [optional] 
**failure** | **str** |  | [optional] 
**retry_count** | **int** |  | 

## Example

```python
from moqentra_client.models.import_job_response import ImportJobResponse

# TODO update the JSON string below
json = "{}"
# create an instance of ImportJobResponse from a JSON string
import_job_response_instance = ImportJobResponse.from_json(json)
# print the JSON string representation of the object
print(ImportJobResponse.to_json())

# convert the object into a dict
import_job_response_dict = import_job_response_instance.to_dict()
# create an instance of ImportJobResponse from a dict
import_job_response_from_dict = ImportJobResponse.from_dict(import_job_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


