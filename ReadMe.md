# Brigade
- [Brigade](#brigade)
  - [How to use](#how-to-use)
  - [FAQ](#faq)
  - [TALON (Transaction Authorization Language for Operations and Nodes)](#talon-transaction-authorization-language-for-operations-and-nodes)
    - [Data Types](#data-types)
    - [Variables](#variables)
    - [Functions](#functions)
    - [ConversionTarget](#conversiontarget)
    - [Predefined Variables](#predefined-variables)
  - [Hints for later](#hints-for-later)


## How to use

## FAQ
Where do I get the Filter from?
Take a look at the RPC API provided by the Blockchain. There will be a description on how to build the JSON RPC call.

## TALON (Transaction Authorization Language for Operations and Nodes)

### Data Types
1. String
2. Number
3. SignedNumber
4. Bool
5. Array
6. Map

### Variables
Variables are defined through the properties, environment events or through certain keywords like the assign function
Variables are marked with '$' and can contain arbitrary data types.

Variables are always local to the event where they are created.
However, by adding the variables either to the keystore or the map allows to make variables persistent
> **Warning:** Make sure to remove the values from the persistent storage after they are used. Otherwise the storage get's bloated.
>

The special variables `$keystore` and `$map` are global and can be used to persistently store variables throughout events or chains.

Preset variables include:
- `$chain_name_block_number` e.g. `$ethereum_block_number` holds the block number of the current event
- `$config_file_prefix_contract` e.g. `$eth2_contract` holds the contract address for each config file. Hint: name config files with `prefix_config.json`

### Functions
Here is a list of all supported functions. Note that not every data type can be used with every function.
When using arrays, every operation itself is a for each operation.
However, by using the `at()` function only one index is used.

1. `Contains()` 
   + `array.contains(value) returns true || false`
   + `string.contains(substring) returns true || false`            
   + Return whether a collection contains an item
   + Not applciable in maps. Use get() instead!
2. `At()`
   + `array.at(index) returns arr[index] || Err`
   + Return the value at a given index
3. `As()`
   + `T.as(ConversionTarget) returns ConvertedValue(T)`
   + Convert a value into the specified type
4. `Slice()`
   + `array.slice(start, end) returns array[start..end]`
   + `string.slice(start, end) returns substring`
   + Return a slice of a collection
5. `Push()`
   + T = array | string
   + `T.push(value) returns new_T || false`
   + push a new item on a collection
6. `Pop()`
   + T = array | string
   + `T.pop() returns last_item`
   + return the last item of a collection
7. `Keccak256` (WIP)
8. `Insert()`
   + `map.insert(key, value) returns true || false`
   + Insert a new value with a key into a map
9.  `Remove()`
    - `map.remove(key) returns true || false`
    - Removes a specified key from a Map
10. `Get()`
    - `map.get(key) returns value`
    - Get the value at the key.
11. `Assign`
    - `assign(variable_name, value)`
    - Create a new variable with a name and a value

> Note: Sometimes functions return strings but the context needs the result to be a boolean. Therefore, string can be compared with a boolean true to evaluate to true: `$str.push(a) && true`
>


### ConversionTarget
The following types can be converted from and to:
1. u256
2. i256
3. string
4. hex

### Predefined Variables
In some cases it is necessary to have environment variables which are stored before execution.
Therefore, running the Tool with the `-p <file_path>` flag can take predefined patterns and deploy the variables at startup.
> Note: The patterns are not updated "during" the execution
>
The file provided is a json file with a single array as root element, containing all comma separated strings for variables.

E.g. 
```json
[
    "$keystore.push(0xaabb223344aaccddee)",
    "$map.insert(key, value)"
]
```

## Hints for later
The name of the config file 'ethereum_config.json' indicates the chain name to be 'ethereum' which is used by the custom functions as prefix.
Each chain needs a folder under functions with the chain name.
Each chain needs a function called get_block_number.json which returns the current block number.
In ethereum_socket.rs is a test which can be used to generate the topic id from the Event

Following Types can be used in type conversions: "u256", "i256", "hex", "string"