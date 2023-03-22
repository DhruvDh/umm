(method_declaration
	(modifiers
        (annotation
            name: (identifier) @annotation
            arguments: (annotation_argument_list)
        )
    )
    name: (identifier) @name
)

(method_declaration
	(modifiers
	(marker_annotation
    	name: (identifier) @annotation)
    )
    name: (identifier) @name
    (#eq? @annotation "Test")
)