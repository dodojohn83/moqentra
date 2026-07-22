# CreateTasksRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**asset_ids** | **List[str]** |  | 

## Example

```python
from moqentra_client.models.create_tasks_request import CreateTasksRequest

# TODO update the JSON string below
json = "{}"
# create an instance of CreateTasksRequest from a JSON string
create_tasks_request_instance = CreateTasksRequest.from_json(json)
# print the JSON string representation of the object
print(CreateTasksRequest.to_json())

# convert the object into a dict
create_tasks_request_dict = create_tasks_request_instance.to_dict()
# create an instance of CreateTasksRequest from a dict
create_tasks_request_from_dict = CreateTasksRequest.from_dict(create_tasks_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


