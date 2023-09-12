# Design

## Objects

Objects can be persistent or volatile.
(how is the difference specified?)

```{.graphviz caption="1" height="50%"}
digraph A {
    node    [shape = record];
    edge    [tailclip = false];
    rankdir = "TB";

    agent   [label = "<obj> Agent|<f1> root"];
    app     [label = "<obj> Application"];
    coll    [label = "<obj> Collection"];    
    dict    [label = "<obj> Dictionary"];
    parent  [label = "<obj> Directory|<p> parent|<s> loc|len|<f1> f1|<f2> f2|...|<fn> fn"];
    dir     [label = "<obj> Directory|<p> parent|<s> loc|len|<f1> f1|<f2> f2|...|<fn> fn"];
    dir2    [label = "<obj> Directory|<p> parent|<s> loc|len|<f1> f1|<f2> f2|...|<fn> fn"];
    file    [label = "<obj> File|<s> loc|<f1> meta|<f2> content"];
    file1   [label = "<obj> File|<s> loc|<f1> meta|<f2> content"];
    object  [label = "<obj> Object"];
    loc     [label = "<obj> Location"];

    coll_meth   [label = "Insert:|At:|RemoveAt:|ForEach:"];
    dict_meth   [label = "Set:At:"];

    agent:obj   -> app          [label=is];  
    dict:obj    -> coll:obj     [label=is];
    file:obj    -> object:obj   [label=is];
    dir:obj     -> dict:obj     [label=is];
    coll:obj    -> object:obj   [label=is];
    app:obj     -> object:obj   [label=is];
    loc:obj     -> object:obj   [label=is];

    coll:obj    -> coll_meth    [label=can];
    dict:obj    -> dict_meth    [label=can];
    
    dir:f1      -> file:obj     [label=ref];
    agent:f1    -> parent:obj   [label=ref];
    dir:fn      -> file1:obj    [label=ref];
    dir:p       -> parent:obj   [label=ref];
    dir2:p      -> dir:obj      [label=ref];
    dir:f2      -> dir2:obj     [label=ref];
    parent:f1   -> dir:obj      [label=ref];
    dir:s       -> loc:obj      [label=ref];
    parent:s    -> loc:obj      [label=ref];
    dir2:s      -> loc:obj      [label=ref];
    file:s      -> loc:obj      [label=ref];
    file1:s     -> loc:obj      [label=ref];

}
```


Beispiel für ein Statediagramm

```{.graphviz caption="2"}
digraph finite_state_machine {
    rankdir=LR;
    size="8,5"

    node [shape = doublecircle]; S;
    node [shape = point ]; qi

    node [shape = circle];
    qi -> S;
    S  -> q1 [ label = "a" ];
    S  -> S  [ label = "a" ];
    q1 -> S  [ label = "a" ];
    q1 -> q2 [ label = "ddb" ];
    q2 -> q1 [ label = "b" ];
    q2 -> q2 [ label = "b" ];
}
```

Strukturen und Pointer

```{.graphviz caption="3"}
digraph structs {
    node [shape=record];
    struct1 [label="<f0> left|<f1> mid&#92; dle|<f2> right"];
    struct2 [label="<f0> one|<f1> two"];
    struct3 [label="hello&#92;nworld |{ b |{c|<here> d|e}| f}| g | h"];
    struct1:f1 -> struct2:f0;
    struct1:f2 -> struct3:here;
}
```
