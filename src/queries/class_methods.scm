(program
  (block_comment)*
  (line_comment)*
  (class_declaration 
      (class_body
          (method_declaration
          	(modifiers)* @modifier
            (marker_annotation)* @annotation
            ((type_identifier)*
             (generic_type)*
             (boolean_type)* 
		     (void_type)* 
			 (array_type)*
             (integral_type)*
             (floating_point_type)*) @returnType
            (identifier) @identifier
		    (formal_parameters) @parameters
            (throws)* @throws
            )
      )
	)
)