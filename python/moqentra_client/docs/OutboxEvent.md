# OutboxEvent


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** |  | [optional] 
**aggregate_type** | **str** |  | [optional] 
**aggregate_id** | **str** |  | [optional] 
**event_type** | **str** |  | [optional] 
**payload** | **object** |  | [optional] 

## Example

```python
from moqentra_client.models.outbox_event import OutboxEvent

# TODO update the JSON string below
json = "{}"
# create an instance of OutboxEvent from a JSON string
outbox_event_instance = OutboxEvent.from_json(json)
# print the JSON string representation of the object
print(OutboxEvent.to_json())

# convert the object into a dict
outbox_event_dict = outbox_event_instance.to_dict()
# create an instance of OutboxEvent from a dict
outbox_event_from_dict = OutboxEvent.from_dict(outbox_event_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


