# CreateImportJobRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**source_url** | **str** |  | 
**target_key** | **str** |  | 
**media_type** | **str** |  | 
**total_bytes** | **int** |  | 
**concurrency** | **int** |  | [optional] 
**deadline_seconds** | **int** |  | [optional] 

## Example

```python
from moqentra_client.models.create_import_job_request import CreateImportJobRequest

# TODO update the JSON string below
json = "{}"
# create an instance of CreateImportJobRequest from a JSON string
create_import_job_request_instance = CreateImportJobRequest.from_json(json)
# print the JSON string representation of the object
print(CreateImportJobRequest.to_json())

# convert the object into a dict
create_import_job_request_dict = create_import_job_request_instance.to_dict()
# create an instance of CreateImportJobRequest from a dict
create_import_job_request_from_dict = CreateImportJobRequest.from_dict(create_import_job_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


