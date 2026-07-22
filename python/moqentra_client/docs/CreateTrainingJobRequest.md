# CreateTrainingJobRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**experiment_id** | **str** |  | 
**project_id** | **str** |  | 
**code_digest** | **str** |  | 
**image_digest** | **str** |  | 
**dataset_version_id** | **str** |  | 
**argv** | **List[str]** |  | 

## Example

```python
from moqentra_client.models.create_training_job_request import CreateTrainingJobRequest

# TODO update the JSON string below
json = "{}"
# create an instance of CreateTrainingJobRequest from a JSON string
create_training_job_request_instance = CreateTrainingJobRequest.from_json(json)
# print the JSON string representation of the object
print(CreateTrainingJobRequest.to_json())

# convert the object into a dict
create_training_job_request_dict = create_training_job_request_instance.to_dict()
# create an instance of CreateTrainingJobRequest from a dict
create_training_job_request_from_dict = CreateTrainingJobRequest.from_dict(create_training_job_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


